#!/usr/bin/env python3
"""GitLab #434 (SEC-G01): assert the launch-monitoring runbook exists and covers every signal.

Verifies ``docs/runbooks/launch-monitoring.md`` is present and documents a command/query for each
of the seven required launch signals (plus the existing reorg-halt alert). Keeps the runbook from
silently losing a signal on a future edit. Exit 0 = complete, exit 1 = missing signal(s).

Run via: ``make check-launch-monitoring-docs``
"""
import sys
from pathlib import Path

DOC = Path(__file__).resolve().parent.parent / "docs" / "runbooks" / "launch-monitoring.md"

# Each signal must be evidenced by at least one of these (case-insensitive) markers in the runbook.
REQUIRED = {
    "contract / indexer error spikes": ["indexer_failed_blocks", "error-level", " error "],
    "indexer lag behind chain tip": ["last_indexed_height", "lag="],
    "API 429 / 5xx rate": ["429", "rate_limit", "tower_governor", "rate-limit"],
    "large swaps": ["swap_events", "return_amount"],
    "large LP withdrawals": ["liquidity_events", "event_type='remove'", "remove"],
    "blacklist hits": ["blacklist-check", "blacklist hit", "compliance"],
    "pause state changes": ["is_paused", '"paused"', "paused:true"],
    "reorg halt alert": ["indexer_reorg_halt", "reorg"],
}


def main():
    if not DOC.is_file():
        print(f"FAIL: {DOC} not found (SEC-G01 / #434 launch-monitoring runbook missing)")
        return 1
    text = DOC.read_text(encoding="utf-8").lower()
    missing = [
        sig for sig, markers in REQUIRED.items()
        if not any(m.lower() in text for m in markers)
    ]
    if missing:
        for sig in missing:
            print(f"FAIL: launch-monitoring runbook does not cover signal: {sig}")
        print(f"\n{len(missing)} required launch signal(s) missing from {DOC.name}.")
        return 1
    print(f"OK: launch-monitoring runbook covers all {len(REQUIRED)} launch signals (SEC-G01 / #434).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
