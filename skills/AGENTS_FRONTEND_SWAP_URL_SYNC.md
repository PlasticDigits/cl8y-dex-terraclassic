# Agent playbook: Swap URL sync, reverse quotes, Share, Create/Trade prefill (GitLab #713)

Audience: third-party agents touching Swap routing, shareable DEX URLs, reverse quotes, `/create` query prefill, or `/trade?from=&to=` resolve.

**Issue:** [GitLab **#713**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/713)  
**Parent inbound parse:** [#711](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/711) / [`AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md`](./AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md) (**Q711-1–Q711-8**)  
**Invariants:** [`docs/frontend.md` § Swap URL sync](../docs/frontend.md#swap-url-sync) (**Q713-1–Q713-10**)

Land as a **new MR on top of !481**. Do not expand !481 after review has started.

## Problem class

#711 applies inbound `from`/`to` once and never writes the bar back. Copying the address after a picker change shares the **inbound** pair. Uniswap `exactField=output` was ignored. Swap had no Share. Create Pair voided `?a=&b=` on purpose (**C542-11**). Trade identity is `/trade/:pairAddr` with no `from`/`to` resolve.

## Invariants (Q713-1–Q713-10)

| ID | Rule |
|----|------|
| **Q713-1** | Picker / flip / amount → canonical `/?from=&to=` via `setSearchParams({ replace: true })`. Amount debounce = `SIM_QUOTE_DEBOUNCE_MS` (350). No history spam. |
| **Q713-2** | Uniswap aliases still apply, then rewrite to `from`/`to` (aliases leave the bar). |
| **Q713-3** | Write follows live state. Apply does not snap the user back to the inbound query. |
| **Q713-4** | `exactField=output` on a **direct factory pair**: You Receive is independent; You Pay is `reverseSimulateSwap` offer; execute is still offer-in + min received. Multihop / wrap-mapper: ignore output field (pay-sided, silent **#489**). No indexer exact-out. |
| **Q713-5** | Swap header **Share** next to Settings (`data-testid="swap-share-link"`). No new `shell-panel*` / `card-glass` (**C653**). Coarse+narrow: Web Share; else `copyToClipboard` (**Link copied**). `AbortError` silent. Never `window.location.href`. |
| **Q713-6** | `/create?a=&b=` prefills catalog or checksum custom. Hostile / native ignored per side. No auto-submit. No runtime HTTP catalog. |
| **Q713-7** | `/trade?from=&to=` `replace`s to `/trade/{uniquePair}`. Zero or multiple matches → ignore. Path stays canonical. Ticket not auto-placed. |
| **Q713-8** | Production cannot share or rewrite a gem. Never honor `?showGems=1`. |
| **Q713-9** | Canonical string compare breaks apply↔write loops. Build URLs with `URL` + `URLSearchParams` only. |
| **Q713-10** | No lecture banners. No `localStorage`. No `0x`→Terra mapping. U1: UST1 Swap/Trade links remain AMM. |

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/swapQueryParams.ts` | `canonicalSwapSearch`, `parseSwapExactField`, `swapDeepLinkPath` |
| `frontend-dapp/src/pages/SwapPage.tsx` | Apply + replace write-back; reverse-quote field; Share |
| `frontend-dapp/src/utils/sharePageLink.ts` | `buildCanonicalSwapShareUrl` (query payload; trader helper still strips search) |
| `frontend-dapp/src/utils/createPairQuery.ts` | `parseCreatePairQuery` / `canonicalCreatePairSearch` |
| `frontend-dapp/src/utils/tradeQueryResolve.ts` | Unique factory pair from Swap-style `from`/`to` |
| `frontend-dapp/src/services/terraclassic/pair.ts` | `reverseSimulateSwap` (direct only) |

## Do / don’t

- **Do** set `appliedSwapQueryKeyRef` to the **canonical** search before/as you `replace`, so self-authored rewrites are not re-applied.
- **Do** reuse `ShareLinkButton`. Desktop: pass `canShare={() => false}` unless `useCoarseNarrowViewport()`.
- **Don’t** fake exact-out via `quoteCw20ViaRouteSolve` / binary search on `/route/solve`.
- **Don’t** share leftover `recipient` / WC / Uniswap keys.
- **Don’t** feed Swap’s factory graph into Create Pair prefill (**C542-8**).
- **Don’t** `navigate` Trade to anything except `/trade/{bech32}`.
- **Don’t** persist query in `localStorage`.

## Regression

```bash
make verify-issue-713
make verify-issue-711
make verify-issue-542
make verify-issue-665
make verify-issue-678
python3 scripts/check_chrome_nesting.py
```

## Related

- [`AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md`](./AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md) — inbound parse (**Q711**)
- [`AGENTS_FRONTEND_SHARE_LINK.md`](./AGENTS_FRONTEND_SHARE_LINK.md) — TS-2 Swap exception
- [`AGENTS_FRONTEND_CREATE_PAIR_PICKER.md`](./AGENTS_FRONTEND_CREATE_PAIR_PICKER.md) — C542-11 prefill
- [`AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md`](./AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md) — reverse quote is not a submit bypass
- [`AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md`](./AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md) — ignore `pool_only` / `hybrid_optimize` from URL
- [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) — no extra Share chrome
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — silent fail-closed (#489)
