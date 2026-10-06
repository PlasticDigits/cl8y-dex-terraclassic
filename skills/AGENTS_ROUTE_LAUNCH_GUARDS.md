# Agent playbook: route launch guards (#1390)

Use when changing router/token settlement, route quotes, solver eligibility or
client preflight. Issue: [#1390](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1390).
Keep [RT1381 funding invariants](AGENTS_ROUTE_LOCAL_TAX.md),
[contract security](../docs/contracts-security-audit.md),
[solver policy](../docs/route-solver.md), and
[review/verification instructions](../docs/qa/issue-1390/README.md) together.

## Invariants

1. **LG1390-1 — logical path.** Validate the initial offer followed by every hop's
   ask. An intermediate ask followed by the next offer is one visit. Reject any
   second visit to a route-tax-capable token, including zero-rate, manager-exempt,
   disabled-launch-feature and cooldown-zero cases. Ordinary CW20 repetition is
   outside this restriction. Capability discovery uncertainty cannot waive it.
2. **LG1390-2 — parity.** Router execute, forward and reverse queries, solver
   eligibility and client funded preflight use the same repeated-capable-asset
   rule. Do not introduce a same-block exception or trust caller-supplied history.
3. **LG1390-3 — independent routes.** The authenticated acquired-token exception
   allows the sell of a just-bought intermediate within one route. It cannot
   survive route cleanup or waive the first boundary of another route. Keep
   trading-enabled and per-wallet cooldown checks, including atomic rollback.
4. **LG1390-4 — owned restoration.** Only the initial input's integer sell residue
   returned to its original payer may skip max-wallet. Authenticate its exact
   token, payer and amount in live router state. It is bounded by the initial
   authorized debit, emitted once, and never supplied by the external hook.
5. **LG1390-5 — acquired value.** Final bought output, purchased intermediate
   residue and delivery to a different recipient stay capped. No general transfer
   exemption, purchased-output relabelling or router-dust spending is permitted.
6. **LG1390-6 — trader identity.** A protocol-exempt address is invalid as a routed
   trader, matching authenticated execution. Public token previews and fresh
   executable preflight reject it. Manager/directory tax exemption remains
   separate and cannot waive launch guards. Omitted trader is an anonymous price
   estimate, not wallet-specific execution approval. Deterministic solver HTTP
   422 stops Swap, wrapped Swap, Trade and invoice quotes; it cannot become a
   client fallback quote or transport retry.
7. **LG1390-7 — compatibility.** The optional original-input refund context field
   defaults to absent. Old router context can never acquire the new cap exception.
   Deploy router/token 1.2.1 together; retain code/hash discovery and pair
   code-id refresh requirements. Unit tests do not establish deployment.
8. **LG1390-8 — factual evidence.** Run feasible checks before skipping. Overall
   **PASS with explicit skips** is allowed for unavailable checks, with concrete
   reasons and unchecked acceptance scope. Executed failed assertions remain
   FAIL. Record revision, UTC time, exact commands and synthetic-only receipts.

## Procedure

Run `make verify-issue-1390` and `make verify-issue-1381` using the dedicated
worktree/test database. See the [QA checklist](../docs/qa/issue-1390/README.md)
for contracts, solver, frontend, test-chain and deployment instructions. Repeat
available tests after edits. Never widen a protocol control to avoid a check.
