#!/usr/bin/env python3
"""Verify fee-discount tier docs match code and deploy scripts (GitLab #198)."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CANONICAL = ROOT / "docs/reference/fee-discount-tiers.md"
FIXTURES = ROOT / "smartcontracts/tests/src/tier_fixtures.rs"
DEPLOY_LOCAL = ROOT / "scripts/deploy-dex-local.sh"
DEPLOY_SOFT_LAUNCH = ROOT / "scripts/lib/mainnet-soft-launch-defaults.sh"
DOCS = ROOT / "docs"

TierRow = tuple[int, int, int, bool]  # tier_id, min_cl8y_balance, discount_bps, governance_only


def parse_canonical_md(path: Path) -> list[TierRow]:
    rows: list[TierRow] = []
    for line in path.read_text().splitlines():
        if not line.startswith("|") or line.startswith("|-"):
            continue
        parts = [p.strip() for p in line.split("|")[1:-1]]
        if len(parts) < 6 or parts[0] == "Tier ID":
            continue
        try:
            tier_id = int(parts[0])
        except ValueError:
            continue
        wei = parts[2].strip("`")
        discount_bps = int(parts[3])
        gov_only = parts[5].strip("`").lower() == "true"
        rows.append((tier_id, int(wei), discount_bps, gov_only))
    if not rows:
        raise ValueError(f"no tier rows parsed from {path}")
    return rows


def parse_fixtures(path: Path) -> list[TierRow]:
    block = re.search(
        r"STANDARD_PRODUCTION_TIERS: &\[TierRow\] = &\[(.*?)\];",
        path.read_text(),
        re.DOTALL,
    )
    if not block:
        raise ValueError(f"STANDARD_PRODUCTION_TIERS not found in {path}")
    rows: list[TierRow] = []
    for m in re.finditer(
        r"\((\d+),\s*([\d_]+),\s*([\d_]+),\s*(true|false)\)",
        block.group(1),
    ):
        tier_id = int(m.group(1))
        min_bal = int(m.group(2).replace("_", ""))
        discount = int(m.group(3).replace("_", ""))
        gov = m.group(4) == "true"
        rows.append((tier_id, min_bal, discount, gov))
    return rows


def parse_deploy_local(path: Path) -> list[TierRow]:
    rows: list[TierRow] = []
    for line in path.read_text().splitlines():
        if "add_tier" not in line:
            continue
        m = re.search(r"'(\{.*\})'", line)
        if not m:
            continue
        payload = json.loads(m.group(1))
        tier = payload["add_tier"]
        rows.append(
            (
                int(tier["tier_id"]),
                int(tier["min_cl8y_balance"]),
                int(tier["discount_bps"]),
                bool(tier["governance_only"]),
            )
        )
    if not rows:
        raise ValueError(f"no add_tier rows in {path}")
    return rows


def parse_soft_launch_defaults(path: Path) -> list[TierRow]:
    """Parse add_tier JSON lines from mainnet_soft_launch_fee_discount_tier_msgs heredoc."""
    text = path.read_text()
    block = re.search(
        r"mainnet_soft_launch_fee_discount_tier_msgs\(\)\s*\{.*?cat\s+<<'EOF'\n(.*?)EOF",
        text,
        re.DOTALL,
    )
    if not block:
        raise ValueError(f"mainnet_soft_launch_fee_discount_tier_msgs heredoc not found in {path}")
    rows: list[TierRow] = []
    for line in block.group(1).splitlines():
        line = line.strip()
        if not line or "add_tier" not in line:
            continue
        payload = json.loads(line)
        tier = payload["add_tier"]
        rows.append(
            (
                int(tier["tier_id"]),
                int(tier["min_cl8y_balance"]),
                int(tier["discount_bps"]),
                bool(tier["governance_only"]),
            )
        )
    if not rows:
        raise ValueError(f"no add_tier rows in {path}")
    return rows


def assert_same(label_a: str, a: list[TierRow], label_b: str, b: list[TierRow]) -> None:
    if a != b:
        print(f"MISMATCH: {label_a} vs {label_b}", file=sys.stderr)
        print(f"  {label_a}: {a}", file=sys.stderr)
        print(f"  {label_b}: {b}", file=sys.stderr)
        sys.exit(1)


def scan_docs_for_drift() -> None:
    """Fail on stale field names or duplicate full tier tables outside canonical doc."""
    tier_table_re = re.compile(
        r"^\|\s*(?:Tier(?:\s+ID)?|0)\s*\|.*\|\s*(?:250|1000|10000)\s*\|",
        re.MULTILINE,
    )
    for md in DOCS.rglob("*.md"):
        if md == CANONICAL:
            continue
        text = md.read_text()
        rel = md.relative_to(ROOT)

        if "min_tokens" in text and "not `min_tokens`" not in text:
            print(f"ERROR: {rel} uses deprecated min_tokens (use min_cl8y_balance)", file=sys.stderr)
            sys.exit(1)

        if tier_table_re.search(text) and "fee-discount" in text.lower():
            print(
                f"ERROR: {rel} contains a fee-discount tier table; "
                f"link to {CANONICAL.relative_to(ROOT)} instead",
                file=sys.stderr,
            )
            sys.exit(1)


def main() -> int:
    canonical = parse_canonical_md(CANONICAL)
    fixtures = parse_fixtures(FIXTURES)
    deploy = parse_deploy_local(DEPLOY_LOCAL)
    soft = parse_soft_launch_defaults(DEPLOY_SOFT_LAUNCH)

    assert_same("canonical md", canonical, "tier_fixtures.rs", fixtures)
    assert_same("canonical md", canonical, "deploy-dex-local.sh", deploy)
    assert_same("canonical md", canonical, "mainnet-soft-launch-defaults.sh", soft)
    scan_docs_for_drift()

    print(
        f"OK: {len(canonical)} tiers aligned across "
        "docs/reference/fee-discount-tiers.md, tier_fixtures.rs, "
        "deploy-dex-local.sh, mainnet-soft-launch-defaults.sh"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
