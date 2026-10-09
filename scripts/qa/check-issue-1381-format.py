#!/usr/bin/env python3
"""Check all changed indexer Rust files without rewriting unrelated legacy tests."""
import subprocess
import tomllib
from pathlib import Path

files = set()
for args in (
    ["diff", "--name-only", "-z", "origin/main...HEAD", "--", "indexer"],
    ["diff", "--name-only", "-z", "--", "indexer"],
    ["diff", "--cached", "--name-only", "-z", "--", "indexer"],
    ["ls-files", "--others", "--exclude-standard", "-z", "indexer"],
):
    files.update(name.decode() for name in subprocess.check_output(["git", *args]).split(b"\0")
                 if name.startswith(b"indexer/") and name.endswith(b".rs"))
if files:
    edition = tomllib.loads(Path("indexer/Cargo.toml").read_text())["package"]["edition"]
    subprocess.run(["rustfmt", "--check", "--edition", edition, "--config", "skip_children=true",
                    *sorted(files)], check=True)
print(f"PASS formatting: {len(files)} changed indexer Rust files")
