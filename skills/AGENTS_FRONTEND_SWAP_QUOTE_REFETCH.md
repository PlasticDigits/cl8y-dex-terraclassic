# Swap / Trade sim quote refetch & Calculating… (GitLab #484)

Use this skill when changing **Swap** or **Trade market** sim `useQuery` options, receive-field loading copy, indexer `route/solve` timeouts, or anything that can leave the UI stuck on **Calculating…**.

Companion: [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) (route row + submit–quote alignment #356/#360). Product docs: [`docs/frontend.md`](../docs/frontend.md#submit-quote-alignment--calculating-ux).

## Problem class

Distant CW20 pairs (e.g. mainnet **JADE → RUBY**, no direct pair) use the indexer multihop path:

1. `GET /api/v1/route/solve` (hybrid `global_v1` — often **>15s**, many LCD hybrid queries)
2. LCD enrich / `simulate_swap_operations` / per-hop spread preflight

If `refetchInterval: 10_000` fires while that `queryFn` is still running, React Query’s default **`cancelRefetch`** cancels and restarts the quote. Slow solves never settle → perpetual **Calculating…** on button (`simQuoteStale` ← `isFetching`) and, previously, on the Swap receive field (`isFetching` alone).

## Invariants

| Invariant | Meaning |
|-----------|---------|
| **No overlapping interval refetches** | Sim queries use [`simQuoteRefetchInterval`](../frontend-dapp/src/utils/quoteDebounce.ts) — return `false` while `fetchStatus === 'fetching'`, else `SIM_QUOTE_REFETCH_INTERVAL_MS` (10s). Do **not** pass a bare `10_000` number. |
| **Receive keeps prior quote** | Swap **You Receive** uses [`shouldShowSimReceiveCalculating`](../frontend-dapp/src/utils/quoteDebounce.ts): Calculating only when `isFetching && !simData`. Background refetch keeps the last amount (`keepPreviousData`). Trade market already kept the amount visible. |
| **Submit still blocks while fetching** | [`isSubmitQuoteStale`](../frontend-dapp/src/utils/quoteDebounce.ts) treats `isFetching` as stale (#356). Button may show Calculating during a legitimate in-flight refresh; it must not loop forever because of cancel/restart. |
| **Route solve timeout budget** | Indexer `getRouteSolve` / `postRouteSolve` use `INDEXER_ROUTE_SOLVE_TIMEOUT_MS` (**45s** prod; **4s** E2E outage mode) — longer than default `INDEXER_FETCH_TIMEOUT_MS` (**15s**). |
| **Caller AbortSignal** | Swap `simQuery.queryFn` passes React Query `signal` into `getRouteSolve` so superseded keys abort without stacking HTTP work. |

## Code map

| Concern | Location |
|--------|----------|
| Refetch interval + receive Calculating helpers | [`quoteDebounce.ts`](../frontend-dapp/src/utils/quoteDebounce.ts) — `simQuoteRefetchInterval`, `shouldShowSimReceiveCalculating` |
| Unit tests | [`quoteDebounce.test.ts`](../frontend-dapp/src/utils/quoteDebounce.test.ts) |
| Swap sim query | [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) — `refetchInterval: simQuoteRefetchInterval`, `queryFn({ signal })` → `getRouteSolve(…, { signal })` |
| Trade market sim query | [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) — same `refetchInterval` helper |
| Indexer timeouts + signal merge | [`indexer/client.ts`](../frontend-dapp/src/services/indexer/client.ts) — `INDEXER_ROUTE_SOLVE_TIMEOUT_MS`, `mergeTimeoutSignal` |
| Client tests | [`indexer/__tests__/client.test.ts`](../frontend-dapp/src/services/indexer/__tests__/client.test.ts) |

## Do / don’t

- **Do** keep submit gated on `isFetching` (stale-submit safety).
- **Do** show prior receive amount during background refetch when `simData` exists.
- **Don’t** restore `refetchInterval: 10_000` without the fetching guard.
- **Don’t** lower route-solve timeout below production distant-pair latency without an indexer-side speedup.
- **Don’t** treat Keplr mobile alone as the root cause — it amplifies LCD latency; the state machine bug is web-wide.

## Regression checklist

1. Stable amount on Swap: after first quote settles, receive amount stays visible during the next background refresh; submit may briefly disable until fetch completes.
2. Amount change: typed ≠ debounced → Calculating on button (#356); receive may show prior placeholder until new quote lands.
3. Multihop / slow indexer: quote completes (or fails to **Quote unavailable**) without infinite Calculating; no cancel/restart every 10s while still fetching.
4. Trade market: same `simQuoteRefetchInterval` behavior.
5. Unit: `quoteDebounce.test.ts` (#484 cases), `client.test.ts` route-solve timeout + AbortSignal.

## Follow-ups (out of scope for the UI hang fix)

- Indexer: tighten hybrid search budget / cache hot distant pairs so JADE↔RUBY-class solves are routinely &lt;15s.
- Optional: thread React Query `signal` through LCD `queryContract` / enrich+preflight fan-out.
- Product: very small `estimated_amount_out` on thin multi-hop paths once quoting settles.
