//! Filesystem traversal utilities: walking trees, filtering, and extension parsing.

use anyhow::Result;
use ignore::{DirEntry, WalkBuilder};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Parse a comma-separated list of file extensions into a lowercase set.
///
/// Examples:
/// - "jpg,png,gif" -> {"jpg","png","gif"}
/// - Whitespace is ignored and entries are lowercased.
pub fn parse_exts(exts: Option<&str>) -> Option<HashSet<String>> {
    exts.map(|s| {
        s.split(',')
            .map(|e| e.trim().to_ascii_lowercase())
            .filter(|e| !e.is_empty())
            .collect::<HashSet<_>>()
    })
}

/// Walks paths respecting .gitignore unless `hidden=true`.
///
/// - `roots`: paths to scan
/// - `hidden`: include hidden files and directories when true
/// - `follow_symlinks`: follow symlinks when true
/// - `min_size`: only include files at least this many bytes
/// - `exts`: optional set of lowercase file extensions to include
///
/// Returns a list of regular file paths that match the criteria.
pub fn collect_files(
    roots: &[PathBuf],
    hidden: bool,
    follow_symlinks: bool,
    min_size: u64,
    exts: Option<&HashSet<String>>,
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
    ent.file_type().map(|ft| ft.is_file()).unwrap_or(false)
}

fn matches_ext(path: &Path, exts: &HashSet<String>) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|e| exts.contains(&e.to_ascii_lowercase()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::parse_exts;

    #[test]
    fn parse_exts_basic() {
        let set = parse_exts(Some(" JPG , png, Gif ,, ")).unwrap();
        assert!(set.contains("jpg"));
        assert!(set.contains("png"));
        assert!(set.contains("gif"));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn parse_exts_empty_items() {
        let set = parse_exts(Some("   , ,  ")).unwrap();
        assert!(set.is_empty());
    }
}
