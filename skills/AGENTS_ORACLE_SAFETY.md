# Agent playbook: oracle integrity and hub containment

For third-party agents touching USD sources, hub valuation, API/adapters or any
financial automation. Issue [#1391](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1391).
Current contract: [O1391 invariants](../docs/indexer-invariants.md#oracle-integrity-and-hub-containment-1391).
Policy, release notes and review checklist: [QA/runbook](../docs/qa/issue-1391/README.md).
Implementation: [PR #1412](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1412);
results/revision: [verification](../docs/qa/issue-1391/verification.md).

1. Apply O1391-1–O1391-10 before legacy X6/H2/H3/H8 rules. Rejected values never
   become zero, a peg, a last-good current quote or fabricated healthy provenance.
2. Keep canonical contract identity/decimals and independent USTC/LUNC CEX sources.
   `vfdusd` is FDUSD/USD; Venus rate is separately pinned, never a DEX volume oracle.
3. Never self-value hub TVL as independent backing or reset reference automatically.
   Preserve fixed reference state on restarts, outages and divergence; bootstrap
   and confirmed switching remain non-oracle-grade spot observations.
4. Use `oracle_safety.py`/`oracle-safety.sh` for live script inputs; require source,
   age, fresh quorum and revalidate before preparing transactions. Numeric overrides
   are dry-run only. Preserve explicit strategy seed constants as strategy.
5. Run `make verify-issue-1391` against isolated Postgres; rerun after any fixes.
   Record exact revision/time/commands/results. Review current API cache expiry and
   propagation through overview, volume/fee USD and adapters, not just parsers.
6. An agent can skip unavailable verification with explicit reasons and report
   overall PASS with skips if every executed check passed. An executed failure is
   FAIL. Unavailable holds/gates cannot prevent implementation review; deployment
   and live manipulation remain unverified until actually executed.

Related: [external oracle](AGENTS_INDEXER_EXTERNAL_ORACLE.md),
[independent reference scheduling](AGENTS_PROVIDER_RECOVERY.md) and its
[bounded cadence runbook](../docs/runbooks/indexer-provider-recovery.md),
[hub USD](AGENTS_INDEXER_HUB_USD.md),
[local Postgres](AGENTS_LOCAL_POSTGRES_DEV.md),
[UST1 LP actions](AGENTS_REBALANCE_MINT_UST1_LP.md),
[cUSTC buyback](AGENTS_MINT_SWAP_CUSTC_UST1.md),
[UST1 buyback](AGENTS_MINT_SWAP_BURN_UST1_CLUNC.md).
