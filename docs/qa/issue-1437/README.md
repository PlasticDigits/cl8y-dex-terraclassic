# #1437 review and verification checklist

[Issue and acceptance criteria](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1437)
· [agent playbook](../../../skills/AGENTS_LISTING_VIRTUAL_PAIRS.md)
· [invariants](../../indexer-invariants.md#listing-only-native-markets-1437)
· [CG/GT contract](../../CG_CMC_COMPLIANCE.md#listing-only-native-markets-1437)
· [receipt](verification.md).

An agent should create an isolated git worktree, read the issue and all comments,
apply LV1437-1–LV1437-10, and provision disposable PostgreSQL using
[indexer database isolation](../../../skills/AGENTS_INDEXER_DATABASE_ISOLATION.md).
Never reset a shared database or migrate with the runtime identity. No chain or
production deployment is needed for this issue.

Run `make verify-issue-1437`. It builds tests once, runs them and retests the same
executables, checks listing-only source boundaries and documentation crosslinks, and verifies
that recovery preserves its explicitly supplied database URL without printing
credentials. Its source fingerprint covers Rust, migrations and recovery/verifier
scripts and must stay unchanged during the run.
Then run the existing listing regression targets below. Save revision, UTC time,
commands, outcomes, and the reason for each unavailable item in a receipt.

| Criterion | Agent procedure / expected result |
|---|---|
| AC1 | Query CG pairs/tickers: five stable opaque ids, factory JSON keys, exact pinned currencies, 24h raw volume assignments, newest real projected price, consolidated pool-only extension, cache hit unchanged. Pagination slices the combined pair list; real ticker page order stays unchanged. |
| AC2 | Wrap five native units with a separate fee: GT sides are principal five, peg price one. Unwrap reverses direction. Crosses preserve source fills, reversed asset order and 6/18 USDT scaling. |
| AC3 | Remove assets/source/mapper or zero custody/reserves: affected market and its events disappear while list requests return 200. Excluded sources and spoofed addresses cannot become legs. |
| AC4 | Request virtual book/trades, including maximal depth, `limit=1`, `limit>500` and buy/sell filters. Existing shapes and ms/seconds timestamps remain. Mock LCD receives zero requests. Unknown ticker 404, malformed ticker 400. |
| AC5 | Query GT pair/asset: existing pair wrapper, `dexKey=cl8y`, correct leg ids and native/pinned-USDT CoinGecko ids. Wrapped or spoofed contracts do not inherit native ids. |
| AC6 | Compare factory feed before/after projection: real events/reserves/indexes identical; virtual swaps carry real tx/sender/block and unique combined event indexes. Exact factory cap 5000 permits extra projections; 5001 returns 400 before projections. |
| AC7 | Query ordinary pair list/detail/candles/trades, CMC, route solve and DeFiLlama; assert no opaque markets. Factory/economic tables, TVL and trader-volume sums do not change. Ordinary-id guards and discovery reject prefix variants before SQL/LCD. |
| AC8 | Seed spoofed symbols and a real LUNC/USTC ticker on different contracts: real ticker wins, virtual CG market/lookup omitted. Repeat requests and cache hits cannot replace factory identities or create extra swaps. |
| AC9 | Run `make verify-issue-685 verify-issue-646 verify-issue-684 verify-issue-694 verify-issue-631 verify-issue-655`. Preserve listing exclusions, cap behavior, stamped reserves, DeFiLlama and pair-liquidity totals. |
| Ingestion | Retest height-attested configured treasury ownership, NULL/rotated backing and print isolation, height URL cache keys, strict principal parsing, wrong emitter, malformed fields, fee-only events, real sender, source ordinal, height attestation, replay uniqueness, postevent custody and reorg cleanup. Run amount/DB/chain regression checks when changing their boundaries. |
| Documentation | Verify all five ids, projection provenance, isolation, crosslinks and the prepared GT form remark. This issue does not submit the form or deploy. |

## Explicit skip policy

Complete every runnable check first and fix/retest failures. An agent may skip a
check it cannot complete after investigating prerequisites: record `SKIP`, the
specific unavailable capability, attempted command, and unverified criterion.
Set `VERIFY_ISSUE_1437_SKIP_DB_REASON` or `VERIFY_ISSUE_1437_SKIP_RUST_REASON` only
for an actual unavailable prerequisite, never to suppress a test failure.

When all executed checks pass, an agent may mark overall verification **PASS WITH
SKIPS** despite unavailable items. Leave skipped acceptance boxes unchecked and
state the evidence scope. Executed failures remain **FAIL** until fixed and
retested. No hold on unavailable external work prevents publishing the tested
implementation PR. Local PASS is not proof of production rollout or crawler
acceptance; neither is required here.

Subsequent production evidence and repeatable operator checks are recorded in
[production rollout and listing receipt](production-listing.md). The original
implementation receipt retains its dated scope.
