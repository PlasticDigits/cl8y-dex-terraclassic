# ADR 0007: Remaining GET `/route/solve` failure modes (retail census)

## Status

Proposed ([#1265](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1265))

Research / decision only. This ADR does **not** change indexer ranking, hop caps, cache buckets, wrap-mapping, Swap/Trade quote branches, wrap-mapper, or CosmWasm.

## Context

Retail “routing is wrong / quote is insane / Calculating forever” is several mechanisms. Closed or sibling tickets already own pieces:

| Ticket | What it owns |
|--------|----------------|
| [#1218](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1218) (landed, PR 1297) | Native wrap-enter GET after client wrap-map; leftover LocalTerra Route wrap-then-cUSTC is [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) |
| [#1257](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1257) (closed, in tree) | Mixed 18/6 hop honesty (`ImplausibleHop`) + 18-dec display + ≥99% theater chrome |
| [#1264](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1264) (code on `main`, PR 1293; issue stays open) | USTC→USTR wrap+2hop **execute gas** — not quote ranking. AC1 `gas_used` unmeasured; leftover [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) must **not** close it |
| [#484](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/484) / [#485](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/485) (closed) | Calculating hang + distant-pair progress |
| [#209](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/209) / [#323](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/323) | Shipped solver; hop cap 4; top-5 shortest |
| [#369](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/369) / [#493](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/493) / [#585](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/585) / [#615](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/615) | Zero-reserve skip, empty-book short-circuit, freeze hop skip, net ranking |
| [#501](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/501) / [#596](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/596) / [#1280](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1280) | Trade=Swap GET helper; always-on hybrid; hop-0-only declared hybrid |

Those did not produce a **single remaining-failure memo**. This census re-read current quote branches and solver constants on **2026-09-21** and ran quote-only public GET probes (contract ids + hop fields only). Wallet LCD sim vs `estimated_amount_out` is **unmeasured** here.

**Out of this census (keyword overlap on “route” / “solve” is not enough):**

- [#1203](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1203) — parallel **split** routing. Feature spike.
- [#690](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/690) / [#691](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/691) — other-DEX v2 LP / IBC USDC intermediate. Extra edges.
- [#1222](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1222) / [#1264](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1264) — execute **gas**, not ranking.
- [#1256](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1256) — typeable You Receive / reverse-quote UX.

## Decision

**Stay.** Document the census. **No impl spawn.**

Filed tickets stay owners of **F0** (#1218), **F1** (#1257), **F10** (#1264 / #1222). Confirmed unfiled F2–F9 either **Kill** (code already honest / same helper) or stay **in-scope of `OPTIMALITY_SCOPE`** without a listed CW20 pair that proves a 6th-path loser.

| ID | Gap class | Score | Why |
|----|-----------|-------|-----|
| **F0** | Native wrap-enter / unwrap-exit skip solve | **Confirm landed — #1218** | Swap wrap-maps then GET `/route/solve` on CW20 ids (PR 1297). GET `token_in=uluna` still **400**. Execute stays pool-only (**H596-7**). Leftover LocalTerra is [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300). Do not spawn ranking work here. |
| **F1** | Mixed 6/18 humanize / sim scale | **Confirm landed — #1257** | `ImplausibleHop` + unique-symbol 18-dec display + ≥99% hide. Dated 1 USTR→USDT is 5.47% (not theater). |
| **F2** | Hop-count top-K drops a better longer CW20 path | **Unmeasured — no spawn** | Shortest-first top-5 is the shipped bound. Hub probes: `search_truncated=false`. No listed CW20 pair in this census proved a cheaper path **outside** K. Do not bump `MAX_PATH_CANDIDATES` here. |
| **F3** | ~100% hop spread still wins | **Kill as unfiled** | Scale-mismatch drain is #1257. Honest whale 99% still quotes; dApp ≥99% theater chrome blocks submit (#678). Dated hub quotes were 0–7% `slippage_percent`. |
| **F4** | `AMOUNT_CACHE_BUCKET` aliases 18-dec vs 6-dec | **Kill** | Bucket is 1e6 **raw**. Unit test: 1 USTR (`1e18`) ≠ 10000 USTR ≠ 1 UST1 (`1e6`). Dated 1 USTR vs 100 USTR used different hops / amounts (3-hop vs 4-hop). Micro-alias of nearby 6-dec sizes is documented cache behavior. |
| **F5** | Client timeout → BFS disagrees with indexer | **Confirm residual — #484/#485** | GET timeout **45s**; progress exists. Distant JADE→RUBY sample **~1.1s** (gem; retail-hidden #562). Swap **may** fall back to client BFS after timeout/4xx/5xx; Trade falls back to pair LCD only. Hang is owned. Fail-closed-on-timeout is not spawned without a dated timeout+wrong-path probe. |
| **F6** | Discovery/POST first-BFS used as retail amount quote | **Kill** | Amount quotes call GET **with** `amount_in`. Discovery GET (no amount) is Advanced compare only. POST is Advanced declared `hybrid_by_hop`, not first-BFS discovery for You Receive. |
| **F7** | Frozen / paused / zero-reserve hop still scored | **Confirm freeze/#369; pause unmeasured** | Frozen pairs omitted from adjacency; frozen-only → **404** (#585). Zero-reserve candidates skipped (#369). Pause is execute-gated, not graph-filtered — **unmeasured** live paused pair; no spawn. |
| **F8** | Trade market vs Swap quote drift | **Kill** | Both default to `quoteCw20ViaRouteSolve`. Trade has no native wrap path and no client-BFS degrade (pair LCD only). |
| **F9** | Identity / colliding ticker | **Confirm carve-out — #715 / #1257** | Contract / denom identity; unique-symbol USDT. Not a remaining solver class. |
| **F10** | Execute gas on wrap+N-hop | **Out of census** | #1264 / #1222. Quote OK vs broadcast OOG is not ranking. |

**No impl spawn.** A later bug/feature issue would need exact token ids, human sizes, native-wrap vs CW20 solve, observed vs honest path (or an explicit product decision to raise K), files to change, and tests. F2 does not have that listed-pair proof. Do not recommend other-DEX hops (#690) or path-split (#1203) as the remaining-failure fix. Do not attach hybrid to native wrap+multihop execute (**H596-7**).

## Invariants (R-CENSUS)

| ID | Meaning |
|----|---------|
| **R-CENSUS-1** | One ADR covers quote-path inventory + F0–F10 — not split across wrap vs CW20 vs Trade tickets. |
| **R-CENSUS-2** | Each retail family has a dated public solve summary (ids + hops + amounts) or an explicit **unmeasured** row. Do not recycle #1218 LUNC→USTR numbers as proof for unrelated CW20 pairs. |
| **R-CENSUS-3** | F0–F10 scored; product decision is Stay vs spawn. This ADR: **Stay**. |
| **R-CENSUS-4** | #1203, #690, #1222, #1264 stay out of this census. |
| **R-CENSUS-5** | Research MR is docs-only: no production `best_execution.rs` / `route_paths.rs` / `route_solver.rs` / `cw20RouteSolveQuote.ts` / `SwapPage.tsx` quote-branch diff. |
| **R-CENSUS-6** | No follow-up implement issue unless the decision is not Stay **and** the gap is not already #1218 / #1257 / #1264. |
| **R-CENSUS-7** | Native wrap+multihop stays pool-only (**H596-7**). Official dApp keeps hybrid on (**H596**). Solver remains advisory within `OPTIMALITY_SCOPE` (top-5 shortest simple paths). |
| **R-CENSUS-8** | Identity is contract / denom, not ticker (#715). Gems stay hidden from retail (#562). Fail-closed: do not paint theater You Receive; do not skip #678 5–30–99 / blacklist / pause / freeze. |

Playbook: [`skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md`](../../skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md). Verify: `make verify-issue-1265`.

## How retail quoting works (2026-09-21 code)

1. **CW20 pay + CW20 receive (Swap default and Trade market default).** `quoteCw20ViaRouteSolve` → `GET /api/v1/route/solve` with `amount_in` (timeout **45s**) → strip interior declared hybrid (#1280) → wallet `simulateMultiHopSwap` is authoritative You Receive → hop preflight → humanize via `getDecimals` / `fromRawAmount` (USTR/USDT pinned 18).
2. **Native pay or receive (Swap only).** Direct 1:1 wrap/unwrap: mapper `simulateNativeSwap`. Else the client wrap-maps `uluna`→cLUNC / `uusd`→cUSTC and **GET** `/route/solve` on the CW20 ids ([#1218](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1218) / **F0** landed). Indexer GET still 400s raw `token_in=uluna`. Submit strips `hybrid` / `book_input` (**H596-7**). Leftover LocalTerra Route wrap-then-cUSTC is [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300).
3. **Advanced typed book.** `POST /route/solve` with caller `hybrid_by_hop` (first BFS path + override). Not the retail empty-book amount quote.
4. **Indexer timeout / 404 / 502.** Swap may fall back to 1-hop LCD or client BFS (can disagree with top-K). Trade falls back to pair `simulateSwap` only (`clientRoute: null`).
5. **Discovery GET (no `amount_in`).** First BFS only — Advanced “Compare indexer route”, not You Receive.

Indexer graph nodes are **CW20 `assets.contract_address` only**. Winner among enumerated paths is max `estimated_amount_out_net` (#615). Cache key: `solver_version` \| tokens \| amount bucket \| maker-fills bucket \| `discount_bps` \| tax identity. Trader address is not keyed.

## Retail quote inventory

| Family | Hits optimized GET? | Wrap-mapped? | Decimals | Last observed class |
|--------|---------------------|--------------|----------|---------------------|
| CW20↔CW20 Swap default | **Yes** (`quoteCw20ViaRouteSolve`) | N/A | Per token; USTR/USDT 18 | Honest within `OPTIMALITY_SCOPE` (dated 1-hop / 3-hop samples) |
| CW20↔CW20 Trade market | **Yes** (same helper) | N/A | same | **F8 Kill** — lockstep with Swap |
| Native wrap 1:1 | **No** | Mapper | 6 native / 6 wrap CW20 | **F0** — not a solver miss |
| Native wrap-enter / unwrap-exit N-hop | **No** | Client BFS after wrap CW20 | 6 / 18 mix on hub | **F0 / #1218**. CW20-substituted GET is a **different** call (see probes) |
| Advanced manual book | POST, not GET optimizer | N/A | — | **F6 Kill** for retail amount quote |
| Swap indexer degrade | Attempted then abandoned | Client BFS / LCD | — | **F5** residual; Trade does not BFS |
| Discovery GET no `amount_in` | First BFS | N/A | — | Not retail amount quote (**F6 Kill**) |
| Frozen-only market | GET 404 | N/A | — | **F7** freeze shipped |
| Gem distant (JADE-family) | Yes if ids passed | N/A | 6 | Retail-hidden (#562). Latency sample ~1s, not 45s abort |

## Dated measurements

Quote-only `GET /api/v1/route/solve` on **2026-09-21** against the public product indexer (`indexer.dex.cl8y.com`). Fields only: token ids, hops, amounts, `quote_kind`, `paths_considered`, `search_truncated`, `slippage_percent`, elapsed. No wallet sim. Gems are **not** a retail acceptance path.

Token ids (columbus-5):

| Label | Contract / denom | Decimals |
|-------|------------------|----------|
| LUNC | `uluna` | 6 |
| cLUNC | `terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg` | 6 |
| cUSTC | `terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch` | 6 |
| UST1 | `terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72` | 6 |
| USTR | `terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv` | 18 |
| USDT | `terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4` | 18 |
| JADE | `terra1ejq3mjjgnklpa3pg4jterlfwsny055gpmcjf3fz0ev3ueajnzeysz6xxgr` | 6 |
| RUBY | `terra1fga508hzx8dd7x8q4uhm6mdhkqv6fxrtsea3r27smdqmv5k2jgxq5zk9fc` | 6 |

| Sample | `amount_in` | HTTP | Elapsed | `quote_kind` | Hops | `paths_considered` / truncated | `estimated_amount_out` | `slippage_percent` | Notes |
|--------|-------------|------|---------|--------------|------|--------------------------------|------------------------|--------------------|-------|
| Native `uluna`→USTR | 1e6 | **400** | 0.88s | — | — | — | — | — | Body `token_in not found in indexer assets`. **F0** solver boundary. |
| UST1→cUSTC (6/6 1-hop) | 1e6 (1 UST1) | 200 | 1.21s | `indexer_hybrid_db` `global_v4` | **1** pair `terra1ceprj…ccw55f` | **5** / false | 169635290 | **0.00** | Direct hub. Honest 1-hop. |
| USTR→USDT (mixed 18/6) | 1e18 (1 USTR) | 200 | 0.88s | `indexer_hybrid_db` `global_v4` | **3** USTR→UST1→cLUNC→USDT | **2** / false | 6733825127434703 | **5.47** | Not theater. #1257 class, not a new F1. Wallet sim: **unmeasured**. |
| USTR→USDT | 1e20 (100 USTR) | 200 | 0.93s | `indexer_hybrid_db` `global_v4` | **4** (via UST1→cUSTC→cLUNC) | **2** / false | 661286480886616024 | **7.17** | Different path vs 1 USTR — **F4 Kill** (no 18/6 bucket alias). |
| cLUNC→USTR (wrap-sub graph) | 1e6 | 200 | 1.00s | `indexer_pool_db` `global_v4` | **2** cLUNC→UST1→USTR | **4** / false | 6919701695911434 | **5.43** | What GET would return **if** wrap-enter mapped to cLUNC. Native Swap still does not call this (#1218). Not recycled as unrelated CW20 proof. |
| USTR→cUSTC (unwrap-sub) | 1e18 | 200 | 0.96s | `indexer_hybrid_db` `global_v4` | **2** USTR→UST1→cUSTC | — | 1253966 | — | CW20 substitute for unwrap-exit. Native receive still **F0**. |
| JADE→RUBY (distant gem) | 1e6 | 200 | **1.10s** | `indexer_pool_db` `global_v4` | **3** | **3** / false | 2893 | omitted | **F5** hang not reproduced. Retail hides gems (#562). |

Wallet `simulateMultiHopSwap` vs these `estimated_amount_out` values: **unmeasured**.

## Attack / abuse evaluation (no exploits)

| ID | Vector | What this census shows | Fail if |
|----|--------|------------------------|---------|
| **A1** Theater quote | Implausible You Receive as a screenshot “price” | #1257 skip + ≥99% hide/block. Dated USTR→USDT is 5.47%, not 10^9-scale. | Treats 10^9-scale out as “deep liquidity” |
| **A2** Cache poison | Bucket alias across decimals / tiers | Key includes tokens + tax/discount identity; 1 USTR ≠ 1 UST1. Trader address not keyed (same-tier share is #283). | Shares quotes across tax/discount identity |
| **A3** Path spoof | Route chip ≠ submitted ops | Display follows `router_operations` after hop-0 strip (#1280 / #450). Fallback BFS is labeled client-fallback on Swap (#329). | Recommending showing a path the tx will not take |
| **A4** LCD / progress DoS | Progress poll or solve fan-out | Existing `RATE_LIMIT_LCD_HEAVY_RPS` **10**; progress does not run hybrid grids. | New unmetered progress/solve endpoint |
| **A5** Frozen hop | Frozen `code_id` still routed | Adjacency skip; frozen-only **404** (#585). | Recommending quoting frozen-only markets |
| **A6** Hybrid on wrap | Attach book to native wrap+N-hop | **H596-7**. Stay keeps pool-only native execute. | Recommending hybrid native execute to “fix ranking” |

## Test plan (research paths)

| ID | Path | Result |
|----|------|--------|
| **P1** Inventory | Swap + Trade market quote branches | CW20 amount → shared GET helper. Native → BFS after wrap. Discovery/POST not You Receive. |
| **P2** Wrap-enter | Native LUNC → hub CW20 | Still **F0** / #1218. GET `uluna` 400. Do not fix here. |
| **P3** Mixed 18/6 | Listed USTR → listed USDT | Still **F1** / #1257 owner. Dated 5.47% / 7.17%. Not folded into a new ticket. |
| **P4** CW20 4-hop | Two listed 6-dec CW20s with a longer cheaper path | **Unmeasured** as a 6th-path loser. UST1→cUSTC considered 5, truncated false, 1-hop won. |
| **P5** Timeout | Distant pair (JADE-family class) | ~1.1s JSON; client abort is 45s. Progress endpoint exists (#485). |
| **P6** Cache | 1 vs 100 human on 18-dec offer | Distinct amounts and hop counts. **F4 Kill**. |
| **P7** Slippage chrome | Any ~100% hop-spread winner | Not observed on hub samples. ≥99% hide + submit-disable remain #678 / #1257. |
| **P8** Trade lockstep | Same ids on `/` vs Trade market | Same `quoteCw20ViaRouteSolve`. **F8 Kill**. |

Negative: a memo that only says “routing is bad” fails. Implementing #1218 or #1257 inside this research MR fails **R-CENSUS-5**. Recommending other-DEX hops (#690) or path-split (#1203) as the remaining-failure fix fails **R-CENSUS-4**. Adding GWT / `ready` / `/agent implement` to #1265 fails (research).

## Consequences

- Agents treat native wrap-enter “wrong path” as **#1218**, not a new ranking ticket.
- Agents treat USTR→USDT scale/theater as **#1257** (landed). Do not spawn a sibling from 5% hub slippage.
- Top-5 shortest remains a **documented non-goal** of global optimality. Raising K needs its own implement issue with a listed-pair proof.
- Swap client-BFS after indexer failure stays a **degrade path**, not a reason to disable hybrid (**H596-5**).
- Wrap+N-hop OOG stays **#1264** / [ADR 0004](./0004-terraclassic-retail-gas-census.md). Post-merge Coolify/LocalTerra for wrap-enter is leftover [ADR 0008](./0008-post-merge-leftover-1287-1298.md) / [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) — **Stay** / no solver spawn unchanged.

## Links

- Issue: [git.cl8y.com #1265](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1265)
- [ADR 0002](./0002-global-best-execution-route-solver.md) — solver contract
- [`docs/route-solver.md`](../route-solver.md) — optimality scope, cache, progress
- [`docs/indexer-invariants.md`](../indexer-invariants.md) — GET global best execution + mixed 18/6
- [`skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md`](../../skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md)
- [`skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](../../skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md)
- [`skills/AGENTS_HYBRID_QUOTING.md`](../../skills/AGENTS_HYBRID_QUOTING.md)
- [`skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md`](../../skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md) (**H596-7**)
- [`skills/AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md`](../../skills/AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md) (**F5**)
- [`skills/AGENTS_FRONTEND_SWAP_USTR_USDT_SCALE.md`](../../skills/AGENTS_FRONTEND_SWAP_USTR_USDT_SCALE.md) (**F1**)
- Verify: `make verify-issue-1265` · QA [`docs/qa/issue-1265/README.md`](../qa/issue-1265/README.md)
