//! Orchestration of the deduping pipeline: parse → collect → group → hash → aggregate → output → actions.

mod actions;
mod cli;
mod fs_utils;
mod grouping;
mod hashing;
mod output;

use actions::{apply_actions, ActionStats};
use anyhow::{Context, Result};
use bytesize::ByteSize;
use clap::Parser;
use std::collections::BTreeMap;
use std::fs;
use std::time::Instant;

use cli::Opts;
use fs_utils::{collect_files, parse_exts};
use grouping::group_by_size;
use hashing::{full_hash, quick_hash};
use output::{print_human, print_json, DuplicateGroup};

#[derive(serde::Serialize)]
struct Timings {
    discover_ms: u64,
    size_group_ms: u64,
    quick_hash_ms: u64,
    full_hash_ms: u64,
    actions_ms: u64,
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    let exts = parse_exts(opts.ext.as_deref());
    let t0 = Instant::now();

    let files = collect_files(
        &opts.paths,
        opts.hidden,
        opts.follow_symlinks,
        opts.min_size.as_u64(),
        exts.as_ref(),
    )
    .context("collecting files failed")?;
    let t1 = Instant::now();

    if files.is_empty() {
        println!("No files matched criteria.");
        return Ok(());
    }

    let total_size: u64 = files
        .iter()
        .map(|p| fs::metadata(p).map(|m| m.len()).unwrap_or(0))
        .sum();

    // Human-only section: don't print in JSON or summary-json modes
    let human_mode = !opts.json && !opts.summary_json;
    if human_mode {
        if opts.list {
            for p in &files {
                println!("{}", p.display());
            }
            println!();
        }

        if !opts.quiet {
            println!(
                "Matched {} files (>= {}) totaling {}",
                files.len(),
                ByteSize(opts.min_size.as_u64()),
                ByteSize(total_size)
            );
        }
    }

    // In JSON/summary modes, do not print any human text to stdout.
    // Warnings/timings still go to stderr via eprintln!.

    // Stage 1: by size
    let by_size = group_by_size(&files);
    let t2 = Instant::now();

    // Validate quick-bytes: clamp to [1 KiB, 1 GiB]
    let mut limit = opts.quick_bytes.as_u64();
    const MIN_QB: u64 = 1024; // 1 KiB
    const MAX_QB: u64 = 1024 * 1024 * 1024; // 1 GiB
    if limit < MIN_QB {
        eprintln!(
            "--quick-bytes too small ({}); clamping to {}",
            ByteSize(limit),
            ByteSize(MIN_QB)
        );
        limit = MIN_QB;
    } else if limit > MAX_QB {
        eprintln!(
            "--quick-bytes too large ({}); clamping to {}",
            ByteSize(limit),
            ByteSize(MAX_QB)
        );
        limit = MAX_QB;
    }

    let mut groups: Vec<DuplicateGroup> = Vec::new();
    let mut reclaimable: u64 = 0;

    // Stage 2: by quick hash (for all size buckets)
    let mut quick_buckets: Vec<(u64, BTreeMap<String, Vec<&std::path::PathBuf>>)> = Vec::new();
    for (size, paths) in by_size.iter().filter(|(_, v)| v.len() > 1) {
        let mut by_qh: BTreeMap<String, Vec<&std::path::PathBuf>> = BTreeMap::new();
        for p in paths {
            match quick_hash(p, limit) {
                Ok(h) => by_qh.entry(h).or_default().push(p),
                Err(e) => eprintln!("quick-hash failed {}: {}", p.display(), e),
            }
        }
        quick_buckets.push((*size, by_qh));
    }
    let t3 = Instant::now();

    // Stage 3: by full hash (for all quick-hash buckets)
    for (size, by_qh) in quick_buckets.into_iter() {
        for (_qh, bucket) in by_qh.into_iter().filter(|(_, v)| v.len() > 1) {
            let mut by_fh: BTreeMap<String, Vec<&std::path::PathBuf>> = BTreeMap::new();
            for p in bucket {
                match full_hash(p) {
                    Ok(h) => by_fh.entry(h).or_default().push(p),
                    Err(e) => eprintln!("full-hash failed {}: {}", p.display(), e),
                }
            }

            for (_fh, dupes) in by_fh.into_iter().filter(|(_, v)| v.len() > 1) {
                let members: Vec<_> = dupes.into_iter().cloned().collect();
                reclaimable = reclaimable
                    .saturating_add(size * ((members.len() as u64).saturating_sub(1)));
                groups.push(DuplicateGroup::new(size, members));
            }
        }
    }
    let t4 = Instant::now();

    let files_in_groups: usize = groups.iter().map(|g| g.members.len()).sum();
    if opts.json {
        print_json(&groups);
    } else if opts.summary_json {
        // A single summary JSON object is printed after actions.
    } else {
        // Sort groups by descending reclaimable bytes, then by size, then by first member
        let mut groups_sorted = groups.clone();
        groups_sorted.sort_by(|a, b| {
            let rec_a = a
                .size
                .saturating_mul((a.members.len() as u64).saturating_sub(1));
            let rec_b = b
                .size
                .saturating_mul((b.members.len() as u64).saturating_sub(1));
            rec_b
                .cmp(&rec_a)
                .then_with(|| b.size.cmp(&a.size))
                .then_with(|| {
                    let a0 = a.members.first().map(|p| p.display().to_string()).unwrap_or_default();
                    let b0 = b.members.first().map(|p| p.display().to_string()).unwrap_or_default();
                    a0.cmp(&b0)
                })
        });
        if !opts.quiet {
            print_human(&groups_sorted, reclaimable);
        } else if !groups_sorted.is_empty() {
            println!(
                "Found {} duplicate groups; reclaimable: {}",
                groups_sorted.len(),
                ByteSize(reclaimable)
            );
        }
    }

    // Side effects last, and only on explicit opt-in
    let action_stats: ActionStats = apply_actions(&groups, opts.delete, opts.hardlink, opts.yes);
    let t5 = Instant::now();

    // Emit summary JSON if requested (after actions to include errors and timings)
    if opts.summary_json {
        let timings: Option<Timings> = if opts.timings {
            Some(Timings {
                discover_ms: (t1 - t0).as_millis() as u64,
                size_group_ms: (t2 - t1).as_millis() as u64,
                quick_hash_ms: (t3 - t2).as_millis() as u64,
                full_hash_ms: (t4 - t3).as_millis() as u64,
                actions_ms: (t5 - t4).as_millis() as u64,
            })
        } else {
            None
        };
        let summary = serde_json::json!({
            "groups": groups.len(),
            "files": files_in_groups,
            "reclaimable": reclaimable,
            "errors": action_stats.errors,
            "timings": timings,
        });
        println!("{}", serde_json::to_string_pretty(&summary)?);
    }

    if opts.timings {
        eprintln!(
            "timings: discover_ms={} size_group_ms={} quick_hash_ms={} full_hash_ms={} actions_ms={}",
            (t1 - t0).as_millis(),
            (t2 - t1).as_millis(),
            (t3 - t2).as_millis(),
            (t4 - t3).as_millis(),
            (t5 - t4).as_millis()
        );
    }

    // Non-zero exit code if any action error occurred
    if (opts.delete || opts.hardlink) && action_stats.errors > 0 {
        std::process::exit(1);
    }

    Ok(())
}
