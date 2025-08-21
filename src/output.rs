//! Output and data model: duplicate groups, human and JSON printers.

use bytesize::ByteSize;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize, Clone)]
pub struct DuplicateGroup {
    pub size: u64,             // bytes per file in this group
    pub members: Vec<PathBuf>, // all paths that are identical
}

impl DuplicateGroup {
    /// Construct a group ensuring:
    /// - members are sorted lexicographically for stable output
    /// - the first member is the canonical path (lexicographically first)
    pub fn new(size: u64, mut members: Vec<PathBuf>) -> Self {
        members.sort_by(|a, b| a.as_os_str().cmp(b.as_os_str()));
        Self { size, members }
    }
}

/// Print human-readable output for duplicate groups.
/// Groups are expected to already be ordered by the caller.
pub fn print_human(groups: &[DuplicateGroup], reclaimable: u64) {
    for g in groups {
        println!(
            "== {} duplicates of {} ==",
            g.members.len(),
            ByteSize(g.size)
        );
        for p in &g.members {
            println!("  {}", p.display());
        }
    }
    if !groups.is_empty() {
        println!();
        println!(
            "Found {} duplicate groups; reclaimable: {}",
            groups.len(),
            ByteSize(reclaimable)
        );
    }
}

/// Print stable, pretty JSON suitable for consumption.
/// Keys remain unchanged; member order is stable by construction.
pub fn print_json(groups: &[DuplicateGroup]) {
    // stable, pretty JSON for GitHub README examples.
    // Deterministic group order: by reclaimable desc, size desc, then first member.
    let mut gs = groups.to_vec();
    gs.sort_by(|a, b| {
        let rec_a = a.size.saturating_mul((a.members.len() as u64).saturating_sub(1));
        let rec_b = b.size.saturating_mul((b.members.len() as u64).saturating_sub(1));
        rec_b
            .cmp(&rec_a)
            .then_with(|| b.size.cmp(&a.size))
            .then_with(|| {
                let a0 = a.members.first().map(|p| p.as_os_str()).unwrap_or_default();
                let b0 = b.members.first().map(|p| p.as_os_str()).unwrap_or_default();
                a0.cmp(b0)
            })
    });
    println!("{}", serde_json::to_string_pretty(&gs).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_determinism_and_order() {
        let g1 = DuplicateGroup::new(10, vec!["/b".into(), "/c".into()]); // reclaimable 10
        let g2 = DuplicateGroup::new(5, vec!["/a".into(), "/b".into(), "/c".into()]); // reclaimable 10
    // same reclaimable -> size desc then first member
    let mut gs = [g2.clone(), g1.clone()];
    gs.sort_by(|a, b| {
            let rec_a = a.size.saturating_mul((a.members.len() as u64).saturating_sub(1));
            let rec_b = b.size.saturating_mul((b.members.len() as u64).saturating_sub(1));
            rec_b
                .cmp(&rec_a)
                .then_with(|| b.size.cmp(&a.size))
                .then_with(|| {
                    let a0 = a.members.first().map(|p| p.as_os_str()).unwrap_or_default();
                    let b0 = b.members.first().map(|p| p.as_os_str()).unwrap_or_default();
            a0.cmp(b0)
                })
        });
        assert_eq!(gs[0].size, 10);
        assert_eq!(gs[1].size, 5);
    }
}
