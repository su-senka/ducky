use anyhow::{Context, Result};
use bytesize::ByteSize;
use clap::{ArgAction, Parser};
use ignore::{DirEntry, WalkBuilder};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about = "Find candidate files for deduplication")]
struct Opts {
    /// Paths to scan
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    /// Minimum file size to consider (e.g. 256KB, 1MB)
    #[arg(long, default_value = "1KB")]
    min_size: ByteSize,

    /// Only include files with these extensions (comma-separated, no dots)
    #[arg(long)]
    ext: Option<String>,

    /// Include hidden files
    #[arg(long)]
    hidden: bool,

    /// Follow symlinks
    #[arg(long)]
    follow_symlinks: bool,

    /// List matching files (otherwise just prints a summary)
    #[arg(long, short = 'l', action = ArgAction::SetTrue)]
    list: bool,
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    let exts = parse_exts(opts.ext.as_deref());

    let files = collect_files(&opts.paths, opts.hidden, opts.follow_symlinks, opts.min_size.as_u64(), exts.as_ref())
        .context("collecting files failed")?;

    if files.is_empty() {
        println!("No files matched criteria.");
        return Ok(());
    }

    let total_size: u64 = files.iter()
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

    Ok(())
}

/// Parse "jpg,png,gif" -> ["jpg","png","gif"]
fn parse_exts(exts: Option<&str>) -> Option<Vec<String>> {
    exts.map(|s| {
        s.split(',')
            .map(|e| e.trim().to_ascii_lowercase())
            .filter(|e| !e.is_empty())
            .collect()
    })
}

/// Walks paths respecting .gitignore unless `hidden=true`.
fn collect_files(
    roots: &[PathBuf],
    hidden: bool,
    follow_symlinks: bool,
    min_size: u64,
    exts: Option<&Vec<String>>,
) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for root in roots {
        let mut wb = WalkBuilder::new(root);
        wb.standard_filters(!hidden);
        wb.follow_links(follow_symlinks);
        for res in wb.build() {
            let ent = match res {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !is_regular_file(&ent) {
                continue;
            }
            let path = ent.path();
            let meta = match ent.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.len() < min_size {
                continue;
            }
            if let Some(extlist) = exts {
                if !matches_ext(path, extlist) {
                    continue;
                }
            }
            out.push(path.to_path_buf());
        }
    }
    Ok(out)
}

fn is_regular_file(ent: &DirEntry) -> bool {
    ent.file_type()
        .map(|ft| ft.is_file())
        .unwrap_or(false)
}

fn matches_ext(path: &Path, exts: &Vec<String>) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|e| exts.contains(&e.to_ascii_lowercase()))
        .unwrap_or(false)
}
