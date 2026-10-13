import { useEffect, useRef } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getRouteSolveProgress } from '@/services/indexer/client'
import type { IndexerRouteExecutionMetadata } from '@/types'
import { isExpiredLocalQuote } from '@/utils/localQuoteFreshness'
import { isLocalQuoteRaw } from '@/utils/localRouteQuote'

/** Observe only the original exact-input quote cohort. Advisory progress can request
 * a fresh validated quote; it never supplies operations, floors or signing permission.
 * Callers disable this while reviewing/signing or using a declared execution plan. */
export function useRouteQuotePromotion(input: {
  enabled: boolean
  intentKey: string
  tokenIn: string
  tokenOut: string
  amount: string
  trader?: string
  maxMakerFills: number
  maxSpread: string
  poolOnly?: boolean
  quote?: { return_amount: string; executionMetadata?: IndexerRouteExecutionMetadata }
  refetch: () => unknown
}): void {
  const { enabled, intentKey, tokenIn, tokenOut, amount, trader, maxMakerFills, maxSpread, poolOnly, quote, refetch } =
    input
  const metadata = quote?.executionMetadata
  const expiry = metadata?.expires_at_ms
  const generation = metadata?.generation
  const active = enabled && !!metadata && !!quote && !isExpiredLocalQuote(quote) && amount !== '0'
  const progress = useQuery({
    queryKey: [
      'routeQuotePromotion',
      intentKey,
      tokenIn,
      tokenOut,
      amount,
      trader,
      maxMakerFills,
      maxSpread,
      poolOnly,
      generation,
      expiry,
    ],
    queryFn: ({ signal }) =>
      getRouteSolveProgress(tokenIn, tokenOut, amount, {
        trader,
        maxMakerFills,
        maxSpread,
        poolOnly,
        signal,
      }),
    enabled: active,
    retry: false,
    staleTime: 0,
    refetchOnWindowFocus: false,
    refetchInterval: (query) => {
      if (
        query.state.fetchStatus === 'fetching' ||
        query.state.status === 'error' ||
        expiry == null ||
        expiry <= Date.now() ||
        query.state.data?.search_complete === true
      )
        return false
      // Original observations expire in at most 20s; no timer can renew that window.
      return Math.min(1_000, expiry - Date.now())
    },
  })
  const attempted = useRef<{ intent: string; best?: string }>({ intent: '' })
  useEffect(() => {
    const identity = JSON.stringify([intentKey, generation, expiry])
    if (attempted.current.intent !== identity) attempted.current = { intent: identity }
    const best = progress.data?.best_amount_out
    if (
      !active ||
      progress.isFetching ||
      !quote ||
      isExpiredLocalQuote(quote) ||
      progress.data?.quote_generation !== generation ||
      !isLocalQuoteRaw(best) ||
      !isLocalQuoteRaw(quote.return_amount) ||
      BigInt(best) <= BigInt(quote.return_amount) ||
      (attempted.current.best != null && BigInt(best) <= BigInt(attempted.current.best))
    )
      return
    attempted.current.best = best
    void refetch()
  }, [active, intentKey, generation, expiry, progress.data, progress.isFetching, quote, refetch])
}
