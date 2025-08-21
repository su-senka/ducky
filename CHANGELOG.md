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

## [0.1.2] - 2025-08-21
### Fixed
- Correct per-phase timings in `--summary-json --timings` and human `--timings` mode.  
  Previously, `discover_ms`, `size_group_ms`, `quick_hash_ms`, and `full_hash_ms` could report the same value due to incorrect checkpointing.

### Notes
- No CLI changes. JSON array schema unchanged. Summary JSON unchanged except for accurate `"timings"`.
