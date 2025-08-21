## [0.1.0] - 2025-08-21
- Initial release
- Features: size → quick-hash → full-hash pipeline
- Output: human, JSON, summary JSON
- Safe actions: --delete, --hardlink (with --yes guard)
- Deterministic output, same-inode guard, cross-device handling

## [0.1.1] - 2025-08-21
### Fixed
- `--json` and `--summary-json` now emit **only JSON** to stdout.  
  Previously, a human summary line was printed before the JSON array/object, 
  which broke tools like `jq`.  
