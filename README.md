# Ducky

Fast, safe, deterministic duplicate file finder written in Rust.

Ducky scans directories for duplicate files using a three-stage pipeline:

1. Group by size — cheap, no I/O.
2. Quick hash — first N bytes (default 64 KB).
3. Full hash — BLAKE3 over the whole file for exact matches.

Output can be human-readable, machine-readable JSON, or a compact summary JSON for scripting.
Optional actions let you delete duplicates or replace them with hard links, with strong safety guards.

---

## Features

- Deterministic results — groups and members are sorted, JSON is stable.
- Fast — efficient traversal, streaming BLAKE3 hashing, optional quick-hash tuning.
- Safe by default — never mutates files without `--yes`.
- Flexible output — human, JSON array, or summary JSON.
- Cross-platform — tested on macOS, Linux, Windows.

---

## Install

With Cargo (from Git):

```bash
cargo install --locked --git https://github.com/su-senka/ducky
```

---

## Usage

Basic scan:

```bash
ducky ~/Photos --min-size 256KB
```

List files:

```bash
ducky ~/Downloads --min-size 1MB --ext jpg,png --list
```

Delete duplicates (keep first in each group):

```bash
ducky ~/Media --delete --yes
```

Replace duplicates with hard links:

```bash
ducky ~/Media --hardlink --yes
```

JSON output (array of groups):

```bash
ducky ~/Docs --json > dupes.json
```

Summary JSON (great for scripts):

```bash
ducky ~/Docs --summary-json --timings
```

Quiet mode (human output without per-group listings):

```bash
ducky ~/Projects --quiet
```

---

## Safety

- Never modifies files without `--yes`.
- Skips files that are unreadable or cross-device (hardlink mode).
- Same-inode guard prevents accidentally deleting the canonical when files are already hard-linked.
- Exit codes:
  - `0` = success (no errors)
  - `1` = completed with action errors (skips are not errors)

---

## Example JSON

```json
[
  {
    "size": 4724736,
    "members": [
      "/Users/alex/Photos/img1.jpg",
      "/Users/alex/Photos/img1_copy.jpg"
    ]
  }
]
```

Summary mode:

```json
{
  "groups": 2,
  "files": 5,
  "reclaimable": 786432000,
  "errors": 0,
  "timings": {
    "discover_ms": 12,
    "size_group_ms": 1,
    "quick_hash_ms": 4,
    "full_hash_ms": 18,
    "actions_ms": 0
  }
}
```
