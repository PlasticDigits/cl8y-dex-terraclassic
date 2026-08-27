# Agent playbook: hide soft-launch gems from production retail UI (GitLab #562)

Audience: third-party agents touching Swap/Trade/Pool/Charts/Create pickers, `findRoute` / `route/solve` display, Coolify faucet env, or gem classification.

**Issue:** [GitLab **#562**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562)  
**Invariants:** [`docs/frontend.md` § Production hide of test tokens](../docs/frontend.md#production-hide-test-tokens) (**P562-1–P562-8**)  
**Related:** [#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534) rank-only (gems still listed on LocalTerra), [#542](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/542) Create Pair gems, [#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547) `/pool` table, [#501](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/501) hybrid `route/solve`, [#473](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/473) faucet Mint, [#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631) DeFiLlama volume/fee gem exclude ([`AGENTS_DEFILLAMA.md`](./AGENTS_DEFILLAMA.md) — keep `COLUMBUS5_GEM_ADDRESSES` in lockstep)

## Problem class

columbus-5 still has eight noneconomic CW20s (EMBER…PEARL) and ten gem pairs. Ranking them last ([#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534)) left them in empty browse, typed search, Swap defaults, and economic quotes that could hop RUBY. Production `dex.cl8y.com` must look like a finished economic DEX. This is **discovery + quoting**, not on-chain deletion.

## Do / don’t

- **Do** gate browse with `retailExposeTestTokens()` = `VITE_NETWORK !== 'mainnet'` **or** `VITE_SHOW_TEST_TOKENS=true` (build-arg only — **never** `?showGems=1`).
- **Do** identify gems by **hardcoded columbus-5 addresses** (`COLUMBUS5_GEM_ADDRESSES`, includes QUARTZ/PEARL) **and** `GEM_SYMBOLS`. Do not rely only on Coolify `VITE_TOKEN_*`.
- **Do** filter tokens/pairs in the shared helpers (`filterRetailDiscoveryTokens` / `filterRetailDiscoveryPairInfos` / `filterRetailDiscoveryIndexerPairs`) so Swap, Trade, Limits, `/pool` (including column sort), Charts, and Create Pair stay consistent.
- **Do** drop test pairs from the BFS graph and reject `route/solve` hops that include a gem when both endpoints are economic (`shouldRejectGemBridgeQuote` — fail closed, do not sanitize the Route row).
- **Do** keep `/trader`, `/limits` history, and LP rows showing gem balances. `/portfolio` Open Positions / header P&amp;L / Recent activity hide gems by default with **Show test pairs** ([#674](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/674)). Deep link `/trade/<gem-pair>` may still open (optional one-line “Legacy noneconomic market.”).
- **Don’t** burn, factory-delete, or wipe indexer gem rows.
- **Don’t** treat UST1 / CL8Y / wrap as gems (**U6** / **P534-8**). Address in the gem set wins over a spoofed `symbol=UST1` (**X1**).
- **Don’t** rewrite LocalTerra / swarm / Playwright onto UST1 just to hide gems. `VITE_NETWORK=local` still lists them and **Test pairs**.
- **Don’t** add an always-on “test tokens removed” banner. Absence is the UX.
- **Don’t** hide Mint by client list alone: production Coolify must **unset** `VITE_FAUCET_ADDRESS` (**F11**). Recommend faucet **Pause** + optional `RemoveMinter` (**F9**).

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/pairCatalogRank.ts` | `retailExposeTestTokens`, `COLUMBUS5_GEM_ADDRESSES`, filters, `shouldRejectGemBridgeQuote`, Swap defaults |
| `frontend-dapp/src/utils/tokenSearchQuery.ts` | Swap empty browse / typed search |
| `frontend-dapp/src/utils/pairSearchQuery.ts` | Pair local fallback search |
| `frontend-dapp/src/components/trade/PairSearchSelect.tsx` | Trade/Limits empty browse + indexer hits |
| `frontend-dapp/src/utils/poolListQuery.ts` | `/pool` catalog + column/search pages |
| `frontend-dapp/src/utils/createPairTokenCatalog.ts` | Skip `gems:` append on production |
| `frontend-dapp/src/services/terraclassic/router.ts` | `findRoute` drops gem pairs unless exit hatch |
| `frontend-dapp/src/utils/cw20RouteSolveQuote.ts` | Reject gem-bridge hybrid quotes |

## Rank vs hide

- **LocalTerra / QA override:** #534 still applies — economic first, gems last, **Test pairs** divider.
- **Production:** gems are omitted (divider has nothing to group). Typed `RUBY` / `EMBER` returns no gem rows.

## Regression

```bash
make verify-issue-562
```

Vitest: `pairCatalogRank.issue562.test.ts`, token/pair search filters, PairSearchSelect production RTL, Create Pair catalog, `findRoute` / `quoteCw20ViaRouteSolve` gem-bridge reject, plus Swap/Pool/Trade page RTL. Local regression: `make verify-issue-534` (and #542 / #547 / #481) still use `VITE_NETWORK=local`. Playwright P1 (`e2e/retail-test-tokens-562.spec.ts`, Swap pay still lists EMBER) runs when `make has-localterra` and `frontend-dapp/.env.local` exist. Stacked post-merge: `make verify-issue-573` ([#573](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/573)).

## Related

- [`AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md`](./AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md) — `/portfolio` performance hide + toggle ([#674](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/674))
- [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) — empty-browse rank when gems **are** listed
- [`AGENTS_FRONTEND_TOKEN_SEARCH.md`](./AGENTS_FRONTEND_TOKEN_SEARCH.md) — Swap factory gate (#481)
- [`AGENTS_FRONTEND_CREATE_PAIR_PICKER.md`](./AGENTS_FRONTEND_CREATE_PAIR_PICKER.md) — Create Pair listed CW20s
- [`AGENTS_FRONTEND_POOL_TABLE.md`](./AGENTS_FRONTEND_POOL_TABLE.md) — `/pool` catalog
- [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md) — displayed hops = executed hops
- [`AGENTS_SOFT_LAUNCH_FAUCET.md`](./AGENTS_SOFT_LAUNCH_FAUCET.md) — Mint nav **F11** + Pause **F9**
- [`docs/runbooks/soft-launch-faucet.md`](../docs/runbooks/soft-launch-faucet.md) — operator Pause
- [`AGENTS_POST_MERGE_STACK.md`](./AGENTS_POST_MERGE_STACK.md) — Coolify+indexer cut with !368–!377 ([#573](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/573))
