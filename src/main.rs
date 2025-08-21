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

use cli::Opts;
use fs_utils::{collect_files, parse_exts};
use grouping::group_by_size;
use hashing::{full_hash, quick_hash};
use output::{print_human, print_json, DuplicateGroup};

fn main() -> Result<()> {
    let opts = Opts::parse();
    let exts = parse_exts(opts.ext.as_deref());
    let t0 = std::time::Instant::now();

    let files = collect_files(
        &opts.paths,
        opts.hidden,
        opts.follow_symlinks,
        opts.min_size.as_u64(),
        exts.as_ref(),
    )
    .context("collecting files failed")?;
    let t_discover = t0.elapsed();

    if files.is_empty() {
        println!("No files matched criteria.");
        return Ok(());
    }

    let total_size: u64 = files
        .iter()
        .map(|p| fs::metadata(p).map(|m| m.len()).unwrap_or(0))
        .sum();

    if opts.list {
        for p in &files {
            println!("{}", p.display());
        }
        println!();
    }

    println!(
        "Matched {} files (>= {}) totaling {}",
        files.len(),
        ByteSize(opts.min_size.as_u64()),
        ByteSize(total_size)
    );

    // Stage 1: by size
    let by_size = group_by_size(&files);
    let t_group_size = std::time::Instant::now();

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

    let t_qh_start = std::time::Instant::now();
    for (size, paths) in by_size.iter().filter(|(_, v)| v.len() > 1) {
        // Stage 2: by quick hash
        let mut by_qh: BTreeMap<String, Vec<&std::path::PathBuf>> = BTreeMap::new();
        for p in paths {
            match quick_hash(p, limit) {
                Ok(h) => by_qh.entry(h).or_default().push(p),
                Err(e) => eprintln!("quick-hash failed {}: {}", p.display(), e),
            }
        }

        // Stage 3: by full hash
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
                    .saturating_add((*size) * ((members.len() as u64).saturating_sub(1)));
                groups.push(DuplicateGroup::new(*size, members));
            }
        }
    }
    let t_fullhash = std::time::Instant::now().duration_since(t_qh_start);

    let files_in_groups: usize = groups.iter().map(|g| g.members.len()).sum();
    if opts.json {
        print_json(&groups);
    } else if opts.summary_json {
        // Only a single summary JSON object will be printed later after actions.
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
    let t_actions_start = std::time::Instant::now();
    let action_stats: ActionStats = apply_actions(&groups, opts.delete, opts.hardlink, opts.yes);
    let t_actions = t_actions_start.elapsed();

    // Emit summary JSON if requested (after actions to include errors and timings)
    if opts.summary_json {
        let timings = if opts.timings {
            Some(serde_json::json!({
                "discover_ms": t_discover.as_millis(),
                "size_group_ms": t_group_size.elapsed().as_millis(),
                "quick_hash_ms": t_qh_start.elapsed().as_millis(),
                "full_hash_ms": t_fullhash.as_millis(),
                "actions_ms": t_actions.as_millis(),
            }))
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
            t_discover.as_millis(),
            t_group_size.elapsed().as_millis(),
            t_qh_start.elapsed().as_millis(),
            t_fullhash.as_millis(),
            t_actions.as_millis()
        );
    }

    // Non-zero exit code if any action error occurred
    if (opts.delete || opts.hardlink) && action_stats.errors > 0 {
        std::process::exit(1);
    }

    Ok(())
}
