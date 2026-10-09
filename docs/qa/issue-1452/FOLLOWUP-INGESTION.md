# Incremental admission and authoritative actor recovery

Follow [LQ1452-1–LQ1452-10](../../../skills/AGENTS_LOCAL_SWAP_QUOTES.md),
[the solver contract](../../route-solver.md#local-quote-contract-1452), and
[the agent verification procedure](README.md). This follow-up extends the
implementation evidence in [RESULTS.md](RESULTS.md); it does not establish
production deployment or complete issue acceptance.

## Invariants and budgets

- Scheduled configuration admission selects eight rotating pairs from the capped
  512-pair/2,048-asset inventory. It discovers only their required assets, rather
  than completing every asset, actor and pair before publishing useful data.
  A completed pair check publishes the compatible, still-fresh cumulative slice.
  Cancelling a pass cannot discard earlier successful publications.
- Admission has one nonoverlapping worker, a 60-second pass deadline and a
  two-second cadence with missed ticks skipped. Every pair's dependent checks
  have a six-second deadline. The LCD background view retains its shared two
  outstanding operations and four provider attempts/second across configuration,
  reserve and wrapper workers. Each namespace also serializes fixture/manual
  ingestion calls; at most 32 admission namespaces exist.
- Each pair retains its original configuration expiry, bounded by its asset
  discovery times. Failed checks remove that pair's pending admission. Publishing
  or refreshing reserves never renews failed or older configuration. Asset-id,
  address, catalog/router/factory policy changes and freeze invalidate retained
  admission. Reserve publications retain the existing atomic configuration epoch
  check. Each quote checks per-hop configuration expiry and pins the route's
  earliest expiry in its execution metadata/cache lifetime; an expired unrelated
  edge cannot disable a still-fresh corridor. Discovery topology and reserve
  refresh skip expired edges. Expired entries are excluded from later publication.
- Each normal pass samples eight rotating recent actors from a capped recent-120
  census plus eight rotating historical blocked actors. At most 2,048 actor
  contexts remain. Missing fees use conservative full fees; missing blacklist
  reads cannot remove known negatives. Capacity overflow immediately quarantines
  quotes, including response-cache hits, without evicting a block.
- Recovery runs only during independent scheduled admission. It scans the
  configured factory's complete raw-state inventory at a single chain height,
  requiring the LCD's explicit response-height attestation on every page.
  A scan is bounded by 128 pages of 64 rows, the LCD's 16-KiB historical response
  cap and a 45-second deadline. Wasmd model keys are hexadecimal; values and
  pagination cursors are base64. The canonical `bl_wallets` prefix and cw2
  `cl8y-dex-factory` version `1.12.0` establish the supported storage layout.
- Strict key ordering, valid encodings, positive boolean blacklist records,
  bounded unique wallets, nonrepeating cursors and a terminal pagination page
  are required. Missing layout/height/page data, an unknown factory version,
  provider errors, timeout or capacity excess preserve quarantine. A complete
  inventory atomically replaces retained negatives, changes generation and
  configuration epoch, invalidates response caches and clears that namespace's
  capacity quarantine. Admission serializes this recovery, and the atomic reset
  requires the same configuration epoch captured before the scan; a newer actor
  observation cannot be erased by an older complete inventory. Global
  namespace-registry overflow remains fail closed.
  Skips and workflow overrides never clear runtime quarantine.
- The hidden `ingest_fixture` helper drains the capped inventory for controlled
  test initialization; production uses `ingest_once` slices. Requests call neither.

The 512-pair boundary is a memory/work limit, not a claim that every market stays
admitted simultaneously under a slow provider. Under sustained delay, older
configuration expires and its routes become explicitly unavailable. This change
allows useful completed slices to initialize without waiting for a full census;
large-factory deployed coverage and provider redundancy remain separate evidence.

## Agent review checklist

- [x] Inspect bounded rotating selection: 64 eight-pair slices cover a 512-pair
  inventory without increasing per-pass selection work.
- [x] Assert publication uses original expiry and excludes expired observations;
  an expired unrelated pair must not disable a fresh route, and emitted metadata
  must pin the route's own earliest configuration expiry.
- [x] Assert retained negatives, rotation and capacity refusal never evict blocks.
- [x] Assert malformed/unknown/incomplete inventory cannot prove recovery.
- [x] Exercise a real provider mock with pinned-height headers; a missing response
  height must fail closed.
- [x] Assert authoritative replacement removes only absent negatives, retains the
  current inventory, changes generation and clears response caches atomically.
- [x] Assert namespace recovery cannot waive global registry overflow, and a
  stale-epoch inventory cannot erase a newer block or clear its quarantine.
- [ ] Run deployed largest-factory ingestion with the configured provider policy;
  record cold first-useful time, slice populations, observations and expirations.
  This source branch is unpublished; production measurements are SKIP.
- [ ] Demonstrate actual factory quarantine recovery through the complete pinned
  inventory on deployed providers. No production quarantine or chain mutation was
  induced for this source check; this acceptance remains SKIP.

Run the focused command below twice, inspect its assertions and then run
`make verify-issue-1452`. Record source revision, UTC time and exact outcomes in
[RESULTS.md](RESULTS.md). An unavailable deployment/provider prerequisite may be
SKIP and overall PASS WITH SKIPS when every executed assertion passes. An executed
failure stays FAIL and requires correction/retest; no skipped acceptance box is
completed.

```bash
CARGO_TARGET_DIR=/home/answorld/repos/cl8y-dex-terraclassic/indexer/target \
  cargo test --manifest-path indexer/Cargo.toml --lib api::local_quote:: \
  -- --test-threads=1
```

## Source receipts

Focused actor-inventory verification passed four tests on 2026-10-09 UTC,
including normal HTTP/provider height handling and the atomic quarantine reset.
The initial complete local-quote focused suite passed 27 tests before adding
those provider/reset and expiry regressions. Shared-source compilation caught an
unfinished hook `Vec` to `Arc` conversion during retest; that run was FAIL and
was corrected by the hook implementation before another run. Final focused verification and retest each passed **42 local-quote tests**, zero
failures/ignored tests, by **2026-10-09T12:40Z**. These include the per-pair expiry
and stale-epoch recovery regressions added during independent review.
The worktree base revision was `c131e63f6fcac6d5bcc85a524105c14ad6f46a9d`.

The raw inventory wire representation is defined by
[the upstream Wasmd Model schema](https://github.com/CosmWasm/wasmd/blob/main/proto/cosmwasm/wasm/v1/types.proto)
and [its AllContractState pagination schema](https://github.com/CosmWasm/wasmd/blob/main/proto/cosmwasm/wasm/v1/query.proto).
The factory namespace/version above come from this repository's factory contract.

## Receive-sizing regression fixture migration

The legacy `api_route_exact_output` corpus now exercises scheduled ordinary and
v1 community-tax admission, then removes all provider responses before quoting.
Every test asserts zero quote-stage provider requests. Disposable fixtures use
compatible pool/book reserve generations, public synthetic actor identities and
independently seeded address-bound economic marks. The mixed 6/18 example pins
its separate economic mark explicitly; reference outage uses a successfully
admitted route with absent marks, rather than relying on unsupported config.

- [x] Exact 10/100 receive targets and repeated cache identity; independent wide
  integer forward arithmetic proves delivered target and amount-minus-one
  minimality.
- [x] Competing paths, unusable reserve generations, independent fair-reference
  rejection and invalid intermediate metadata preserve useful eligible alternates.
- [x] Same actor fee identities use observed tiers; differing trader/sender
  identities retain full fees and pin both identities in execution metadata.
- [x] Gross sell/buy tax sizing is independently rechecked for maximal economic
  input/minimal gross budget; catalog edits cannot rewrite cached observations.
- [x] Sender-only manager exemption and protocol-trader rejection use ingested
  actor flags. Manager exemption never turns protocol actors into eligible traders.
- [x] Independently observed pause/blacklist guards, stale reserves, unsupported
  config and missing independent references fail closed without request RPC.
- [x] Large fair deviation retains the shared warning fields within the explicit
  request's spread constraint. Provider gas failure cannot force remote validation
  or invalidate still-fresh local observations; separate #1422 provider acceptance
  remains separate evidence.

Verification and retest each passed **15 tests** (14 API assertions plus the
common disposable-database URL guard), zero failures/ignored tests, by
**2026-10-09T12:54Z**:

```bash
set -a
source indexer/.env.qa
set +a
CARGO_TARGET_DIR=/home/answorld/repos/cl8y-dex-terraclassic/indexer/target \
  cargo test --manifest-path indexer/Cargo.toml --test api_route_exact_output \
  -- --test-threads=1
```

The first migration run had six executed failures. Corrections used the
compatible reserve writer instead of changing only the pool lane, pinned the
mixed-decimal independent mark, respected the actual L9 constraint, distinguished
fee identities and allowed scheduled actor/sink observations. Those failed runs
were not skips; only the corrected passing verification/retest establishes the
source result above. Production provider, deployment and wallet acceptance remain
unchecked under the explicit [skip policy](README.md#explicit-skip-policy).
