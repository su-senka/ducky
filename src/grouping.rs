//! Grouping utilities: coarse grouping by file size.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// Groups files by their byte size.
/// Returns a map: size → Vec<paths>
pub fn group_by_size(files: &[PathBuf]) -> BTreeMap<u64, Vec<PathBuf>> {
    let mut map: BTreeMap<u64, Vec<PathBuf>> = BTreeMap::new();

    for path in files {
        if let Ok(meta) = fs::metadata(path) {
            let size = meta.len();
            map.entry(size).or_default().push(path.clone());
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::group_by_size;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    #[test]
    fn groups_by_size() {
        let dir = std::env::temp_dir();
        let p1 = dir.join(format!("ducky_test_{}_a", std::process::id()));
        let p2 = dir.join(format!("ducky_test_{}_b", std::process::id()));
        let p3 = dir.join(format!("ducky_test_{}_c", std::process::id()));

        // sizes: 3, 3, 1
        File::create(&p1).unwrap().write_all(b"abc").unwrap();
        File::create(&p2).unwrap().write_all(b"xyz").unwrap();
        File::create(&p3).unwrap().write_all(b"q").unwrap();

        let files: Vec<PathBuf> = vec![p1.clone(), p2.clone(), p3.clone()];
        let map = group_by_size(&files);
        assert_eq!(map.get(&3).map(|v| v.len()).unwrap_or(0), 2);
        assert_eq!(map.get(&1).map(|v| v.len()).unwrap_or(0), 1);

        // cleanup
        let _ = std::fs::remove_file(p1);
        let _ = std::fs::remove_file(p2);
        let _ = std::fs::remove_file(p3);
    }
}
