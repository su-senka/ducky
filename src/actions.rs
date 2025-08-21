//! Side-effectful actions applied to duplicate groups: delete or hardlink.

use crate::output::DuplicateGroup;
use std::fs;
use std::path::Path;

/// Apply --delete or --hardlink on duplicate groups.
/// Keeps the first path in each group as the canonical file.
#[derive(Debug, Default, Clone, Copy)]
pub struct ActionStats {
    pub deleted: usize,
    pub linked: usize,
    pub skipped_same_inode: usize,
    pub skipped_cross_device: usize,
    pub errors: usize,
}

/// Apply the requested action and return stats. Side effects only when `yes` is true.
pub fn apply_actions(groups: &[DuplicateGroup], delete: bool, hardlink: bool, yes: bool) -> ActionStats {
    let mut stats = ActionStats::default();
    if !(delete || hardlink) {
        return stats; // no-op
    }
    if groups.is_empty() {
        eprintln!("No duplicate groups to modify.");
        return stats;
    }
    if !yes {
        eprintln!("Refusing to modify files without --yes.");
        return stats;
    }

    if delete {
        for g in groups {
            if g.members.len() < 2 { continue; }
            let canonical = &g.members[0];
            for dupe in g.members.iter().skip(1) {
                if same_inode(canonical, dupe) {
                    stats.skipped_same_inode += 1;
                    continue;
                }
                match fs::remove_file(dupe) {
                    Ok(_) => stats.deleted += 1,
                    Err(e) => {
                        stats.errors += 1;
                        eprintln!("Failed to delete {}: {}", dupe.display(), e);
                    }
                }
            }
        }
    } else if hardlink {
        for g in groups {
            if g.members.len() < 2 { continue; }
            let canonical = &g.members[0];
            for dupe in g.members.iter().skip(1) {
                if same_inode(canonical, dupe) {
                    stats.skipped_same_inode += 1;
                    continue;
                }
                if !same_device(canonical, dupe) {
                    stats.skipped_cross_device += 1;
                    eprintln!(
                        "cross-device: cannot hardlink {} -> {}",
                        dupe.display(),
                        canonical.display()
                    );
                    continue;
                }
                // Replace dupe with a hard link to canonical
                if let Err(e) = fs::remove_file(dupe) {
                    stats.errors += 1;
                    eprintln!("Failed to remove {}: {}", dupe.display(), e);
                    continue;
                }
                if let Err(e) = fs::hard_link(canonical, dupe) {
                    stats.errors += 1;
                    eprintln!(
                        "Failed to hardlink {} -> {}: {}",
                        dupe.display(),
                        canonical.display(),
                        e
                    );
                    continue;
                }
                stats.linked += 1;
            }
        }
    }

    eprintln!(
        "actions: deleted={} linked={} skipped_same_inode={} skipped_cross_device={} errors={}",
        stats.deleted, stats.linked, stats.skipped_same_inode, stats.skipped_cross_device, stats.errors
    );
    stats
}

/// Helper to check whether two paths are on the same device.
/// The current flow relies on `fs::hard_link` errors for feasibility.
#[allow(dead_code)]
fn _same_device(_a: &Path, _b: &Path) -> bool {
    true
}

#[cfg(unix)]
fn same_inode(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let (Ok(ma), Ok(mb)) = (fs::metadata(a), fs::metadata(b)) else { return false };
    ma.ino() == mb.ino() && ma.dev() == mb.dev()
}

#[cfg(not(unix))]
fn same_inode(a: &Path, b: &Path) -> bool { a == b }

#[cfg(unix)]
fn same_device(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let (Ok(ma), Ok(mb)) = (fs::metadata(a), fs::metadata(b)) else { return true };
    ma.dev() == mb.dev()
}

#[cfg(not(unix))]
fn same_device(_a: &Path, _b: &Path) -> bool { true }

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    #[cfg(unix)]
    fn same_inode_guard_delete() {
        let dir = std::env::temp_dir();
        let canon = dir.join(format!("ducky_act_{}_canon", std::process::id()));
        let dupe = dir.join(format!("ducky_act_{}_dupe", std::process::id()));
        {
            let mut f = std::fs::File::create(&canon).unwrap();
            writeln!(f, "hello").unwrap();
        }
        std::fs::hard_link(&canon, &dupe).unwrap();

        let group = DuplicateGroup::new(6, vec![canon.clone(), dupe.clone()]);
        let stats = apply_actions(&[group], true, false, true);
        assert_eq!(stats.deleted, 0);
        assert_eq!(stats.skipped_same_inode, 1);
        assert!(canon.exists());
        assert!(dupe.exists());

        let _ = std::fs::remove_file(canon);
        let _ = std::fs::remove_file(dupe);
    }
}
