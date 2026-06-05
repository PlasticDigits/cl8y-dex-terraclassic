//! Static guardrail against the GitLab #284 idiom (no DB, no async).
//!
//! #284: a defaulted query `limit`/`depth` was clamped **upper-only**
//! (`params.limit.unwrap_or(50).min(200)`), so `limit=-1` / `limit=0` reached Postgres as a
//! negative/zero `LIMIT` and 500'd. The fix is `.clamp(1, MAX)`. This test scans every
//! `src/api/**.rs` and FAILS if any single line contains BOTH `unwrap_or(` and `).min(` — the
//! exact upper-only-clamp-on-a-default shape — so a reintroduction is caught at `cargo test`
//! without needing a live DB.
//!
//! The legitimate bare `.min(` uses in the tree lack `unwrap_or(` on the same line and are
//! therefore NOT flagged:
//!   - `hybrid_orderbook_sim.rs`: `Vec::with_capacity(depth.min(combined.len()))`
//!   - `cg.rs`: `(offset + limit).min(result.len() as i64)` (page slice bound)
//!   - `orderbook_sim.rs`: `(depth as i64).min(LIMIT_BOOK_PAGE_MAX)` (page-limit bound)
//! Each takes a separately-clamped value as input, so the lower bound is enforced elsewhere.
//! Runtime lower-bound behavior is covered by `api_limit_lower_bound.rs` and
//! `api_hooks.rs::hooks_negative_and_zero_limit_clamp_to_one_not_500`.

use std::fs;
use std::path::{Path, PathBuf};

/// Recursively collect every `.rs` file under `dir`.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()));
    for entry in entries {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_upper_only_clamp_on_defaulted_query_limit() {
    // CARGO_MANIFEST_DIR = .../indexer
    let api_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api");
    assert!(api_dir.is_dir(), "expected src/api dir at {}", api_dir.display());

    let mut files = Vec::new();
    rust_files(&api_dir, &mut files);
    assert!(!files.is_empty(), "found no .rs files under {}", api_dir.display());

    let mut offenders = Vec::new();
    for file in &files {
        let src =
            fs::read_to_string(file).unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
        for (i, line) in src.lines().enumerate() {
            // The #284 bug: a default (`unwrap_or(`) clamped upper-only (`).min(`) on one line.
            if line.contains("unwrap_or(") && line.contains(").min(") {
                offenders.push(format!("{}:{}: {}", file.display(), i + 1, line.trim()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "GitLab #284 regression: upper-only `.min(MAX)` clamp on a defaulted query \
         limit/depth re-introduced. Use `.clamp(1, MAX)` so negative/zero values cannot \
         reach Postgres as a negative LIMIT (HTTP 500). Offending line(s):\n{}",
        offenders.join("\n")
    );
}
