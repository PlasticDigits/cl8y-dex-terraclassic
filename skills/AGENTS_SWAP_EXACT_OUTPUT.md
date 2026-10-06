# Agent playbook: bounded receive sizing (#1376)

Implementation: [PR #1378](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1378).
Apply when changing receive intent, solver sizing, reverse arithmetic, or quote failure handling.

Issue: [#1376](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1376).
Invariants: [E1376](../docs/frontend.md#solver-receive-sizing).
API and scope: [route solver](../docs/route-solver.md#exact-output-sizing).
Agent checklist and evidence: [issue 1376 QA](../docs/qa/issue-1376/README.md).

1. Preserve the typed receive target and URL direction, including indexer-only paths. Mapper-only quotes use the shared wrap/unwrap integer inverses. CW20 route sizing uses the server's explicit `amount_out` mode; never repeatedly binary-search the public offer-in endpoint from the browser.
2. Exact-output means sizing an offer-in transaction. Compare supported pool-only candidates within the published bounds. Do not claim atomic exact-output or global optimality, and do not attach unsupported native hybrid splits.
3. Require current authoritative forward output on the chosen operations to cover the net target. Require positive checked fair-reference metadata, retain symmetric 5/30/99 protections and recheck them at mutation entry; do not substitute hop-only spread for missing route fairness. Preserve fee-tier context, buy-tax net versus pre-tax minimum bases, intermediate tax handling, extra-debit balance checks, and mapper wrap/unwrap/burn-tax inverses. Unsupported or stale models must fail closed.
4. Clear executable pay and operations when intent/context changes or a quote fails. Preserve the receive target; expose finite loading, unavailable and retry states while disconnected. Deterministic arithmetic/liquidity/budget failures must not enter automatic retry storms.
5. Pool-only contract reverse reuses checked integer inverse math and validates the offer with forward simulation. Hybrid requests retain a finite full-simulation budget; exhaustion is an error, never a fabricated minimal result.
6. Review and rerun `make verify-issue-1376`, affected #1256/#713/#678/#512/#1280 regressions, contract tests/lint, frontend lint/types, and Postgres API tests. Capture exact revision and command results. Follow the QA runbook for deployment inventory and read-only probes before and after migration.

Reviewers may record **PASS with explicit skips** when completed checks pass and unavailable checks have a precise reason and remaining scope. An executed failing assertion remains FAIL; a skipped check remains unverified. This task permits unavailable gates to be skipped, but does not change runtime wallet, inventory, tax, spread, pause, blacklist or credential protections. Never invent deployment, signing or chain evidence.

Related: [receive inverses](AGENTS_FRONTEND_SWAP_REVERSE_QUOTES.md), [URL sync](AGENTS_FRONTEND_SWAP_URL_SYNC.md), [query parameters](AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md), [hop partition](AGENTS_HYBRID_HOP_OFFER.md), [local Postgres](AGENTS_LOCAL_POSTGRES_DEV.md).

## Live verification and closure waiver

[2026-10-04 live evidence](../docs/qa/issue-1376/live-verification.md)
records both Coolify deployments on `3f93106` and the separate pair inventory.
E1376-9 requires complete factory-count/pointer/cw2 checks; the production
100-target legacy reverse query still fails on 1.18.0. The explicit user
permission to skip unavailable operator-custody work and close supersedes the
older production closure hold for this verification task only. Preserve the
FAIL result and deferred migration scope; never describe it as migrated or
all-green production acceptance. Agent work remains first. Do not request
passphrases or override runtime protection. The optional
[`complete-issue-1376.sh`](../scripts/qa/complete-issue-1376.sh) is read-only by
default; `--migrate` uses the pinned artifact and existing 2-of-3 tool, while
`--dry-run` never signs. Repeat live probes after any future migration;
[the TWAP runbook](../docs/runbooks/pair-twap-uint256-columbus5.md) remains
the state-preservation contract.

- [Reverse-query availability #1389](AGENTS_REVERSE_QUERY_BUDGET.md) — bounded pair/router work, socket-peer exact-output admission and checked Observe; `make verify-issue-1389`; [agent checklist and explicit skip policy](../docs/qa/issue-1389/README.md).
