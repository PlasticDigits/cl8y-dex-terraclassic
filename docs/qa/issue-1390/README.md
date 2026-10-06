# Issue #1390 review and verification

Issue: [#1390](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1390).
Rules: [LG1390-1–LG1390-8](../../../skills/AGENTS_ROUTE_LAUNCH_GUARDS.md),
[RT1381](../../../skills/AGENTS_ROUTE_LOCAL_TAX.md).
Related: [contract security](../../contracts-security-audit.md),
[solver](../../route-solver.md), [frontend](../../frontend.md),
[funding verification](../issue-1381/README.md).
Results: [RESULTS.md](RESULTS.md).

The chosen policy rejects repeated route-tax-capable assets in the logical path
`[first offer, each ask]`. Thus capable A in A→B→A and capable B in A→B→C→B
are rejected independently of rates, features, exemptions and cooldown settings.
The adjacent buy/sell of B in ordinary A→B→C remains one visit.

## Agent procedure

1. Use an isolated worktree, Node 24, Rust and a dedicated Postgres test DB. Follow
   the [Postgres playbook](../../../skills/AGENTS_LOCAL_POSTGRES_DEV.md). Never
   reset shared volumes or compile host indexer files as root in Docker.
2. Run `make verify-issue-1390` (tests and retests) and `make verify-issue-1381`.
   Set `CONTRACTS_CARGO_TARGET_DIR`/`INDEXER_CARGO_TARGET_DIR` to existing host
   caches if needed. Set `INDEXER_CARGO_TEST_DEBUG=0` and
   `INDEXER_CARGO_INCREMENTAL=0` to reduce indexer compiler memory/disk use
   independently of the contract profile. Run frontend lint/build and Rust formatting checks.
3. Build current optimized wasm, supply reviewed `cw20_mintable.wasm`, and run
   the LocalTerra command below. It creates synthetic contracts and accounts,
   refuses other chains and preserves shared volumes. Use `--container` and a
   distinct code-cache file for an isolated LocalTerra instance when the shared
   node is unavailable; do not restart or reset another agent’s node. Repeat it with the same
   checksum-verified code cache and fresh fixtures.
4. Review exact context provenance: original input residue is computed once from
   the initial authorized gross budget; external messages cannot set its label.
   Old contexts deserialize without a refund exemption. Inspect cleanup and
   ensure purchased output/intermediate residue cannot match owned restoration.
5. For deployed acceptance, inspect actual updated router/token code hashes,
   factory whitelist and pair code pins; follow the [coordinated migration
   runbook](../../runbooks/route-local-tax-1381.md). Run authorized bounded
   synthetic execution before claiming deployment. Contract tests are not a
   Columbus-5 receipt.

```bash
make verify-issue-1390
make verify-issue-1381
python3 scripts/qa/localterra-route-tax-1381.py \
  --isolated-owner --artifacts smartcontracts/artifacts --code-cache /tmp/dex-1390-codes.json \
  --evidence-file /tmp/dex-1390-funded.json
python3 scripts/qa/localterra-route-guards-1390.py \
  --fixture /tmp/dex-1390-funded.json --evidence-file /tmp/dex-1390-guards.json
```

Agents may skip any item they cannot complete while marking overall verification
**PASS with explicit skips**. First complete feasible checks and provisioning.
Record the unavailable item, concrete limitation, UTC time, revision and remaining
acceptance scope. Leave skipped criteria unchecked. Never convert an executed
failure to PASS, and never count a skip as deployment acceptance. This overrides
unavailable workflow holds only; economic and authentication controls remain.

The verifier accepts `VERIFY_ISSUE_1390_SKIP_CONTRACTS_REASON`,
`VERIFY_ISSUE_1390_SKIP_INDEXER_REASON`, `VERIFY_ISSUE_1390_SKIP_FRONTEND_REASON`,
`VERIFY_ISSUE_1390_SKIP_STATIC_REASON`, `VERIFY_ISSUE_1390_SKIP_CHAIN_REASON` and
`VERIFY_ISSUE_1390_SKIP_LIVE_REASON`. Supply `VERIFY_ISSUE_1390_CHAIN_COMMAND`
and `VERIFY_ISSUE_1390_LIVE_COMMAND` for available execution checks.

## Acceptance checklist for the reviewing agent

| Criterion | Agent check and evidence |
|---|---|
| AC1 — identical cyclic policy | Run issue1390 contract tests and solver/frontend capability tests. Compare forward/reverse/execute A→B→A and A→B→C→B. Cover guarded token positions and cooldown 0/1/>1, feature disabled, zero rates and manager exemption. Reject repeated capable identity even when tax is waived. |
| AC2 — ordinary routing and independent trades | Execute ordinary intermediate buy/sell, then a second independent same-block route; test prior-height expiry. Exercise separate transaction/atomic multi-message attempts. Inspect acquired state cleanup; supplied trader/history, unauthenticated transfers and duplicate hops must fail. |
| AC3 — owned residue and capped acquisition | Lower max-wallet below existing payer holdings. Original nonzero residue returns without new acquired value; zero residue also succeeds. Purchased final output, intermediate residue and different recipients stay capped. Compare gross debit, router dust, pair credit, sinks and supply before/after rollback. Repeat refunds cannot inflate wallet holdings. |
| AC4 — protocol-exempt preview/preflight | Query Sell/Buy preview for protocol addresses (router/pair/factory/self), then forward/reverse quotes and execution; reject deterministically. Test manager exemption separately. Solver GET/POST/exact-output and browser preflight must fail closed without fallback executable promises. Exercise mocked solver HTTP 422 in CW20 and wrapped-input Swap, Trade and invoices; confirm no client fallback simulation and no network retry. |
| Regression | Run existing issue1381 cooldown, token launch-guard, funded balance/supply/dust and solver/frontend tests twice. Include insufficient gross, trading disabled, minimum-return and settlement-cap failures. Record actual outcomes; do not claim anti-snipe/Sybil resistance or round-trip profitability. |
| Deployment | Probe current code/hash and pair pins, migrate in coordinated order, then obtain synthetic transaction receipts. Leave unverified unless those actions actually ran. |
