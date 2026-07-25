# Swap / Trade sim quote refetch & Calculating… (GitLab #484, #496)

Use this skill when changing **Swap** or **Trade market** sim `useQuery` options, receive-field loading copy, indexer `route/solve` timeouts, or anything that can leave the UI stuck on **Calculating…** / show a **stale You Receive** after pay changes.

Companion: [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) (route row + submit–quote alignment #356/#360). Product docs: [`docs/frontend.md`](../docs/frontend.md#submit-quote-alignment--calculating-ux).

## Problem class

Distant CW20 pairs (e.g. mainnet **JADE → RUBY**, no direct pair) use the indexer multihop path:

1. `GET /api/v1/route/solve` (hybrid `global_v1` — often **>15s**, many LCD hybrid queries)
2. LCD enrich / `simulate_swap_operations` / per-hop spread preflight

If `refetchInterval: 10_000` fires while that `queryFn` is still running, React Query’s default **`cancelRefetch`** cancels and restarts the quote. Slow solves never settle → perpetual **Calculating…** on button (`simQuoteStale` ← `isFetching`) and, previously, on the Swap receive field (`isFetching` alone).

Separately (#496): sim queries use `placeholderData: keepPreviousData`. That correctly preserves the receive amount during **same-input** background refetch (#484), but after a **pay amount / pay token** (query-key) change the placeholder is for the *previous* inputs — the UI must show Calculating / Quoting until the new key settles, not leave the old number looking current.

## Invariants

| Invariant | Meaning |
|-----------|---------|
| **No overlapping interval refetches** | Sim queries use [`simQuoteRefetchInterval`](../frontend-dapp/src/utils/quoteDebounce.ts) — return `false` while `fetchStatus === 'fetching'`, else `SIM_QUOTE_REFETCH_INTERVAL_MS` (10s). Do **not** pass a bare `10_000` number. |
| **Receive keeps prior quote on same inputs (#484)** | Swap **You Receive** / Trade **Expected receive** use [`shouldShowSimReceiveCalculating`](../frontend-dapp/src/utils/quoteDebounce.ts): for an unchanged sim key, Calculating only when `isFetching && !hasSettledQuote`. Background refetch keeps the last amount (`keepPreviousData`). |
| **Receive clears on pay change (#496)** | When pay **amount** is still debouncing (`raw ≠ debounced`) or React Query shows `isPlaceholderData` (prior key via `keepPreviousData` after pay/token/key change), show Calculating / Quoting — never leave the previous pair’s receive amount looking current. Pass `hasSettledQuote = !!data && !isPlaceholderData`. |
| **Submit still blocks while fetching** | [`isSubmitQuoteStale`](../frontend-dapp/src/utils/quoteDebounce.ts) treats `isFetching` as stale (#356). Button may show Calculating during a legitimate in-flight refresh; it must not loop forever because of cancel/restart. |
| **Route solve timeout budget** | Indexer `getRouteSolve` / `postRouteSolve` use `INDEXER_ROUTE_SOLVE_TIMEOUT_MS` (**45s** prod; **4s** E2E outage mode) — longer than default `INDEXER_FETCH_TIMEOUT_MS` (**15s**). |
| **Caller AbortSignal** | Swap `simQuery.queryFn` passes React Query `signal` into `getRouteSolve` so superseded keys abort without stacking HTTP work. |

## Code map

| Concern | Location |
|--------|----------|
| Refetch interval + receive Calculating helpers | [`quoteDebounce.ts`](../frontend-dapp/src/utils/quoteDebounce.ts) — `simQuoteRefetchInterval`, `shouldShowSimReceiveCalculating` (`isPlaceholderData`, `payInputsPending`) |
| Unit tests | [`quoteDebounce.test.ts`](../frontend-dapp/src/utils/quoteDebounce.test.ts) (#484 + #496), [`SwapPage.test.tsx`](../frontend-dapp/src/pages/SwapPage.test.tsx) (receive `data-testid="swap-you-receive"` on amount change) |
| Swap sim query | [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) — `refetchInterval: simQuoteRefetchInterval`, `queryFn({ signal })` → `getRouteSolve(…, { signal })`, receive gate + `swap-you-receive` |
| Trade market sim query | [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) — same `refetchInterval` + receive Calculating/Quoting (#496) |
| Indexer timeouts + signal merge | [`indexer/client.ts`](../frontend-dapp/src/services/indexer/client.ts) — `INDEXER_ROUTE_SOLVE_TIMEOUT_MS`, `mergeTimeoutSignal` |
| Client tests | [`indexer/__tests__/client.test.ts`](../frontend-dapp/src/services/indexer/__tests__/client.test.ts) |

## Do / don’t

- **Do** keep submit gated on `isFetching` (stale-submit safety).
- **Do** show prior receive amount during **same-key** background refetch when a settled quote exists (#484).
- **Do** show Calculating / Quoting when pay amount is debouncing or `isPlaceholderData` is true (#496) — `keepPreviousData` alone must not pin a stale receive number.
- **Don’t** pass `!!simData` alone as `hasSettledQuote` without excluding placeholder data.
- **Don’t** restore `refetchInterval: 10_000` without the fetching guard.
- **Don’t** lower route-solve timeout below production distant-pair latency without an indexer-side speedup.
- **Don’t** treat Keplr mobile alone as the root cause — it amplifies LCD latency; the state machine bug is web-wide.

## Regression checklist

1. Stable amount on Swap: after first quote settles, receive amount stays visible during the next background refresh; submit may briefly disable until fetch completes (#484).
2. Amount change (#496 + #356): typed ≠ debounced → Calculating on **button and You Receive**; after debounce + settle, both show the new quote (not the old receive number).
3. Pay token change (#496): switch pay token → You Receive shows Calculating/Quoting until the new pair quote settles (no leftover from previous pair).
4. Multihop / slow indexer: quote completes (or fails to **Quote unavailable**) without infinite Calculating; no cancel/restart every 10s while still fetching.
5. Trade market: same `simQuoteRefetchInterval` + Expected receive Quoting while pay amount pending / placeholder (#496).
6. Unit: `quoteDebounce.test.ts` (#484 + #496 cases), `SwapPage.test.tsx` receive gate, `client.test.ts` route-solve timeout + AbortSignal.

## Follow-ups (out of scope for the UI hang / stale-receive fixes)

- ~~Indexer: tighten hybrid search budget / cache hot distant pairs so JADE↔RUBY-class solves are routinely &lt;15s.~~ → **#485** ([`AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md`](./AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md))
- ~~Product: live searching progress instead of static Calculating…~~ → **#485**
- ~~Stale You Receive after pay amount/token change~~ → **#496** (this skill)
- Optional: thread React Query `signal` through LCD `queryContract` / enrich+preflight fan-out.
- Product: very small `estimated_amount_out` on thin multi-hop paths once quoting settles.
