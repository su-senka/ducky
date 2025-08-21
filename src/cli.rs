//! CLI option parsing with clap for the ducky deduper.

use bytesize::ByteSize;
use clap::{ArgAction, Parser};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Find candidate files for deduplication")]
pub struct Opts {
    /// Paths to scan
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,

    /// Minimum file size to consider (e.g. 256KB, 1MB)
    #[arg(long, default_value = "1KB")]
    pub min_size: ByteSize,

    /// Only include files with these extensions (comma-separated, no dots)
    #[arg(long)]
    pub ext: Option<String>,

    /// Include hidden files
    #[arg(long)]
    pub hidden: bool,

    /// Follow symlinks
    #[arg(long)]
    pub follow_symlinks: bool,

    /// List matching files (otherwise just prints a summary)
    #[arg(long, short = 'l', action = ArgAction::SetTrue)]
    pub list: bool,

    /// Quick-hash sample size (first N bytes)
    #[arg(long, default_value = "64KB")]
    pub quick_bytes: ByteSize,

    /// Output machine-readable JSON instead of human text
    #[arg(long)]
    pub json: bool,

    /// Emit only a single summary JSON object with aggregate stats
    #[arg(long)]
    pub summary_json: bool,

    /// Quiet human output: suppress per-group listings and print only the final summary
    #[arg(long, short = 'q')]
    pub quiet: bool,

    /// Delete duplicates (keep the first path in each group as canonical)
    #[arg(long, conflicts_with = "hardlink")]
    pub delete: bool,

    /// Replace duplicates with hard links to the canonical file (first path)
    #[arg(long, conflicts_with = "delete")]
    pub hardlink: bool,

    /// Don't ask for confirmation before modifying files
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Print basic timings for each phase to stderr; included in summary JSON when used
    #[arg(long)]
    pub timings: bool,
}
