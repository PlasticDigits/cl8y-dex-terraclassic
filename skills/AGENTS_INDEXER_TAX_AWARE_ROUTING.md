# Agent playbook: tax-aware route/solve ranking (GitLab #615)

Use when changing **`GET /api/v1/route/solve`** winner pick, hybrid GET cache keys, Swap/Trade **You Receive**, or community-tax catalog bps that feed the solver.

This is a **score / display / path-eligibility** layer. Do **not** teach pair/router wasm to size extra-debit (**H-01** / **T592-1**). Official dApp stays on GET best-execution (**#596**).

Parent: classify option 2 / Honest hops — [`AGENTS_COMMUNITY_TAX_ROUTER.md`](./AGENTS_COMMUNITY_TAX_ROUTER.md) (**T592-13** / **#607**). Catalog bps — [`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](./AGENTS_INDEXER_COMMUNITY_TOKENS.md) (**#594**). Hybrid quote=execute — [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#615**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/615) | Ranking contract |
| [`community_tax_rank.rs`](../indexer/src/api/community_tax_rank.rs) | Pure score + 11611 pin + cache identity |
| [`best_execution.rs`](../indexer/src/api/best_execution.rs) | Filter middle-sell + max `net_out` |
| [`cw20RouteSolveQuote.ts`](../frontend-dapp/src/utils/cw20RouteSolveQuote.ts) | You Receive = net; `executeAmountOut` = pre-tax |
| [`communityTaxNetOut.ts`](../frontend-dapp/src/utils/communityTaxNetOut.ts) | Buy-split helper (do not double-count Max) |
| [ADR 0002](../docs/adr/0002-global-best-execution-route-solver.md) | Amendment #615 |
| [`docs/route-solver.md`](../docs/route-solver.md) | Pipeline + cache key |

## Invariants **R615-1–R615-8**

1. **R615-1 — hop sims unchanged.** Pair `HybridSimulation` / router `simulate_swap_operations` inputs stay 1:1 inbound. Tax is not FoT hop math.
2. **R615-2 — raw vs net.** `estimated_amount_out` = **`raw_out`**. `estimated_amount_out_net` = buy-split when `token_out` is catalogued, `buy_bps > 0`, trader not directory-exempt, **and** buy actually fires (pair-direct **or** option-2 hops). Execute / `min_return` use raw.
3. **R615-3 — middle-hop skip (option 2 only).** Drop a path that **sells** a catalogued tax token as a hop offer **and** that token is not `token_in`, only when that token’s wasm taxes router hops. Extra-debit leftover sits on the router — execute would revert.
4. **R615-4 — `token_in` tax.** First-hop sell is eligible. All remaining paths pay the same leftover in `token_in`. Do not re-rank on that leftover; winner is still max ask `raw_out` / `net_out`.
5. **R615-5 — 11611 pin.** Columbus-5 **11611** is Honest hops until `COMMUNITY_TAX_OPTION2_CODE_IDS` or `COMMUNITY_TAX_OPTION2_DATA_HASHES` lists that code id / wasm hash. Do **not** silently mark unmigrated 11611 as option 2. Switch is documented in [`AGENTS_COMMUNITY_TAX_ROUTER.md`](./AGENTS_COMMUNITY_TAX_ROUTER.md).
6. **R615-6 — exempt fail-closed.** Manager-directory `trader` → 0 bps (catalog feature + LCD `IsProtocolExempt`). LCD / unknown → **keep** catalog bps. Do not honor a client `buy_bps` query param.
7. **R615-7 — cache isolation.** Hybrid GET cache key includes tax identity (in/out bps + router-hops-tax flag + exempt bit). Ordinary CW20 (`none`) must not share a winner with a tax token.
8. **R615-8 — retail hybrid stays on.** No `pool_only` to dodge tax (**#596**). Max extra-debit (**C593-9**) is sell-side only — do not shrink You Receive again.

## 11611 / option-2 switch

| Env | Effect |
|-----|--------|
| *(unset)* | All catalogued tokens Honest hops (current main crate / live 11611). Pair-direct buy still splits net. |
| `COMMUNITY_TAX_OPTION2_CODE_IDS=99,100` | Those code ids: skip middle TAX sells; multi-hop buy split applies. |
| `COMMUNITY_TAX_OPTION2_DATA_HASHES=abc…` | Matching wasm hash (optional; catalog does not store hash today). |

After CMM migrate of 11611 to option-2 wasm: add **11611** to `COMMUNITY_TAX_OPTION2_CODE_IDS` (or the new hash). Until then, UST1→TAX→USTR stays eligible.

## Verify

```bash
make verify-issue-615
make verify-issue-607   # classify / Honest hops must not regress
```

## Do not

- Add pair/router FoT balance-delta math.
- Put buy/sell bps in a spoofable query param as the only source.
- Serve a no-tax cached winner to a tax-aware client (or the reverse).
- Double-count sell extra-debit on You Receive (Max already reserved leftover).
