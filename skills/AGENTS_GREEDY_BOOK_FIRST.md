# Agent skill: greedy book-first swap (GitLab #708 / leftovers #709 #710)

## When to use

You touch **pair `Cw20HookMsg::Swap`**, **router `TerraSwap`**, **`match_bids` / `match_asks` / `simulate_match_*`**, **hybrid gas**, or docs that mention Pattern C vs pool-only.

Greedy is an **opt-in** on-chain walk: fill live same-side makers that **strictly beat** the residual pool spot, then dump the leftover offer into the pool. It is **not** the official dApp default (that stays indexer **`GET /route/solve`**, [#501](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/501) / [#596](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/596)). Full split search stays off-chain.

Leftovers from [!480](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/480): [#709](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/709) (query mutex, `remainder_to_pool`, pool-spot overflow) and [#710](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/710) (tax / pause / blacklist / AfterSwap L7).

## Invariants (G1–G14)

| Id | Rule |
|----|------|
| **G1** | `hybrid: None` and `greedy: None` is **pool-only**. The book is not read. Do not flip this to greedy-by-default (owner note on #708 is a **follow-up**, not this MR). |
| **G2** | Caller does **not** pass `pool_input` / `book_input` on greedy. Only `max_maker_fills` + optional `book_start_hint`. |
| **G3** | Stop when the next **priceable** live maker's **net Decimal rate** (price after book taker commission) does **not strictly beat** the pool spot `output/input` after `effective_fee_bps`. Equal is a stop. Integer 1-raw-unit CP dumps floor to 0 on large reserves — rates match the 1-unit *marginal* intent. Unpriceable makers (`price == 0`, no inverse) are skipped (**L18** / **L20**), not a stop. Unpriceable **pool** spot (`Decimal::checked_from_ratio` overflow) is also **Skip**, not a VM panic (**A7** / #709). |
| **G4** | Not the solver. Do **not** search interior splits, grids, or multi-hop on-chain. Greedy ≤ indexer `GET /route/solve`; greedy ≥ pool-only when the book strictly beats the pool (modulo fees/dust). Official dApp stays on the solver. |
| **G5** | `max_maker_fills == 0` **rejects**. Oversize **clamps** to `MAX_MAKER_FILLS_HARD_CAP` (100). |
| **G6** | `book_start_hint` is caller-supplied; missing/stale/wrong-side → head (**L17**). A **live same-side** stale hint starts there; greedy **stop** can then dump the whole offer to the AMM (integrator risk, not auto-fallback to a better head). |
| **G7** | Quote = execute: pair `HybridSimulation` and execute both call `resolve_swap_hybrid_mode(hybrid, greedy)` with **both** fields. Queries are read-only (no parks, no AfterSwap). |
| **G8** | Greedy execute (and router greedy hops) require `belief_price` or `min_return` (same floor as pure-book hybrid). |
| **G9** | Unfilled offer after the walk goes to the AMM. Wasm `greedy_stop=remainder_to_pool` when makers were filled and offer remains (not `empty`). |
| **G10** | Reuse `match_bids` / `match_asks` / `simulate_match_*` — do not fork a second walker. |
| **G11** | New serde shape `GreedySwapParams`. Do **not** overload Pattern C `pool_input=0, book_input=offer`. Setting both `hybrid` and `greedy` **rejects** on pair query, pair execute, router sim, and router execute. |
| **G12** | Router greedy is **optional and explicit** per hop. `hybrid: null` on a hop stays pool-only. Reverse sim (`HybridReverseSimulation` / `ReverseSimulateSwapOperations`) does **not** take `greedy` — pool-only / declared hybrid only (document, do not silently quote greedy). |
| **G13** | Map greedy in `getGasLimitForTx` / swarm `gas.ts`. Unmapped must **throw or use the hybrid envelope**, never silent 600k (#475). Hook string stays `"swap"`. |
| **G14** | Pattern C JSON unchanged: `HybridSimulationResponse.greedy_stop` is `skip_serializing_if` none. Existing Pattern C tests stay green. Pair wasm migrate on columbus-5 is **ops follow-up**, not this MR. |

## Stop reasons (`greedy_stop` wire)

| Variant | Wire | Meaning |
|---------|------|---------|
| `WorseThanPool` | `worse_than_pool` | Next live maker does not strictly beat the pool (G3). Preferred over `empty` when the walk stopped on price even with zero fills. |
| `MaxMakers` | `max_makers` | Hit `max_maker_fills` with offer left. |
| `ScanCap` | `scan_cap` | Hit `MAX_SCAN_STEPS`. |
| `Empty` | `empty` | **No** live same-side maker was filled. |
| `Filled` | `filled` | Entire offer consumed on the book. |
| `RemainderToPool` | `remainder_to_pool` | ≥1 maker filled; leftover offer → AMM (**G9** / #709). Do **not** treat this as `empty`. |

Priority: worse → scan_cap → filled → empty (makers==0) → max_makers → remainder_to_pool.

## Beats (pinned)

Compare **Decimal rates**, not 1-raw-unit integer dumps (those floor to 0 on large reserves). Bid: `price * (1 − taker_bps/10000)`. Ask: `(1/price) * (1 − taker_bps/10000)`. Pool: `(output_reserve / input_reserve) * (1 − pool_fee_bps/10000)` via `checked_from_ratio` (overflow → Skip). Fill only when book net rate **strictly greater**; equal or worse stops (**G3**). Book fills do not change AMM reserves, so the pool spot is stable for the walk.

## Types

- Rust: `dex_common::pair::GreedySwapParams`, `greedy_swap_params`, `greedy_simulation_undiscounted`, `resolve_swap_hybrid_mode`, `GreedyStopReason::RemainderToPool`
- TS: `GreedySwapParams` in [`frontend-dapp/src/types/index.ts`](../frontend-dapp/src/types/index.ts)
- Gas: [`gasLimitForGreedyParams`](../frontend-dapp/src/services/terraclassic/hybridSwapGas.ts)

## Official dApp

Do **not** switch Swap/Trade off `/route/solve` onto greedy. Greedy is for bots/integrators who want a simple on-chain book-first without a caller split.

## Tests

```bash
cd smartcontracts && cargo test -p dex-common greedy_swap -- --nocapture
cd smartcontracts && cargo test -p cl8y-dex-pair greedy -- --nocapture
cd smartcontracts && cargo test -p cl8y-dex-tests greedy_book_first_708 -- --test-threads=1
cd smartcontracts && cargo test -p cl8y-dex-tests greedy_blacklist -- --test-threads=1
cd smartcontracts && cargo test -p cl8y-community-tax-token greedy -- --test-threads=1
make verify-issue-708
make verify-issue-709
make verify-issue-710
```

#710 covers community-tax extra-debit (pair-direct `from` + router `trader` / **T592-13**), pause (**L6**), blacklist maker skip / taker reject (**L19**), AfterSwap book+pool (**L7**), G1 omitted-greedy control.

## Canonical docs

- Issues: [GitLab #708](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/708) · [#709](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/709) · [#710](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/710)
- ADR: [`docs/adr/0001-hybrid-quoting-and-routing.md`](../docs/adr/0001-hybrid-quoting-and-routing.md)
- Product: [`docs/limit-orders.md`](../docs/limit-orders.md)
- Integrators: [`docs/integrators.md`](../docs/integrators.md)
- Audit matrix: [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) **G1–G14**
- Hybrid quoting: [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md)
- Hint security: [`AGENTS_BOOK_MATCH_HINT_SECURITY.md`](./AGENTS_BOOK_MATCH_HINT_SECURITY.md)
- Gas: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
- Community-tax router hops: [`AGENTS_COMMUNITY_TAX_ROUTER.md`](./AGENTS_COMMUNITY_TAX_ROUTER.md) (**T592-13** still applies to greedy `Send+Swap`)
