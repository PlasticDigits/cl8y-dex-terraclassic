#!/usr/bin/env python3
"""Verify route-solver docs match shipped constants in Rust (GitLab #310)."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOC = ROOT / "docs/route-solver.md"
BEST_EXEC = ROOT / "indexer/src/api/best_execution.rs"
ROUTE_SOLVER = ROOT / "indexer/src/api/route_solver.rs"
ROUTE_GRAPH = ROOT / "indexer/src/api/route_graph.rs"
HYBRID_OPT = ROOT / "indexer/src/api/hybrid_route_opt.rs"


def extract_usize(path: Path, name: str) -> int:
    text = path.read_text()
    m = re.search(rf"pub const {re.escape(name)}:\s*usize\s*=\s*(\d+);", text)
    if m:
        return int(m.group(1))
    m = re.search(rf"const {re.escape(name)}:\s*usize\s*=\s*(\d+);", text)
    if m:
        return int(m.group(1))
    m = re.search(rf"const {re.escape(name)}:\s*u32\s*=\s*(\d+);", text)
    if m:
        return int(m.group(1))
    raise ValueError(f"{name} not found in {path}")


def extract_str_const(path: Path, name: str) -> str:
    text = path.read_text()
    m = re.search(
        rf"pub const {re.escape(name)}:\s*&str\s*=\s*\"([^\"]+)\";",
        text,
        re.DOTALL,
    )
    if m:
        return m.group(1)
    m = re.search(
        rf"pub const {re.escape(name)}:\s*&str\s*=\s*\n\s*\"([^\"]+)\";",
        text,
    )
    if m:
        return m.group(1)
    raise ValueError(f"{name} not found in {path}")


def extract_duration_secs(path: Path, name: str) -> int:
    text = path.read_text()
    m = re.search(
        rf"(?:pub )?const {re.escape(name)}: Duration = Duration::from_secs\((\d+)\);",
        text,
    )
    if not m:
        raise ValueError(f"{name} not found in {path}")
    return int(m.group(1))


def extract_u128(path: Path, name: str) -> int:
    text = path.read_text()
    m = re.search(rf"const {re.escape(name)}:\s*u128\s*=\s*([\d_]+);", text)
    if not m:
        raise ValueError(f"{name} not found in {path}")
    return int(m.group(1).replace("_", ""))


def main() -> int:
    doc = DOC.read_text()
    errors: list[str] = []

    solver_version_lcd = extract_str_const(BEST_EXEC, "SOLVER_VERSION_LCD")
    solver_version_db = extract_str_const(BEST_EXEC, "SOLVER_VERSION_DB")
    max_paths = extract_usize(BEST_EXEC, "MAX_PATH_CANDIDATES")
    optimality = extract_str_const(BEST_EXEC, "OPTIMALITY_SCOPE")
    default_hops = extract_usize(ROUTE_SOLVER, "GET_DEFAULT_MAX_HOPS")
    pool_only_hops = extract_usize(ROUTE_SOLVER, "GET_POOL_ONLY_MAX_HOPS")
    cache_ttl = extract_duration_secs(ROUTE_SOLVER, "ROUTE_CACHE_TTL")
    cache_ttl_distant = extract_duration_secs(ROUTE_SOLVER, "ROUTE_CACHE_TTL_DISTANT")
    graph_cache_ttl = extract_duration_secs(ROUTE_GRAPH, "ROUTE_GRAPH_CACHE_TTL")
    cache_max = extract_usize(ROUTE_SOLVER, "ROUTE_CACHE_MAX_ENTRIES")
    amount_bucket = extract_u128(ROUTE_SOLVER, "AMOUNT_CACHE_BUCKET")
    grid_points = extract_usize(HYBRID_OPT, "GRID_POINTS")

    lcd_budget = max_paths * default_hops * (grid_points + 2 * 2 * grid_points)

    checks: list[tuple[str, list[str]]] = [
        ("solver_version_lcd", [solver_version_lcd, "global_v3"]),
        ("solver_version_db", [solver_version_db, "global_v4"]),
        ("max_paths", [str(max_paths), "top-5", "five"]),
        ("optimality", [optimality]),
        ("lcd_budget", [str(lcd_budget), "1700"]),
        ("default_hops", [f"GET_DEFAULT_MAX_HOPS` | {default_hops}", f"≤ {default_hops} hops"]),
        ("pool_only_hops", [f"GET_POOL_ONLY_MAX_HOPS` | {pool_only_hops}", f"max **{pool_only_hops} hops**"]),
        ("cache_ttl", [f"{cache_ttl} s", f"{cache_ttl}s", "12 s (1-hop direct)"]),
        ("cache_ttl_distant", [f"{cache_ttl_distant} s", f"{cache_ttl_distant}s", "90 s"]),
        ("graph_cache_ttl", [f"{graph_cache_ttl} s", f"{graph_cache_ttl}s", "ROUTE_GRAPH_CACHE_TTL"]),
        ("cache_max", [str(cache_max)]),
        ("amount_bucket", [str(amount_bucket), "1_000_000", "1000000"]),
        ("grid_points", [f"{grid_points}-point", f"({grid_points} ", f"= {grid_points}"]),
    ]

    for label, needles in checks:
        if not any(n in doc for n in needles):
            errors.append(f"docs/route-solver.md missing expected mention for {label}: {needles[0]}")

    if optimality not in doc:
        errors.append("docs/route-solver.md must include full OPTIMALITY_SCOPE string")

    if errors:
        for e in errors:
            print(f"ERROR: {e}", file=sys.stderr)
        return 1

    print("OK: route-solver docs align with Rust constants")
    return 0


if __name__ == "__main__":
    sys.exit(main())
