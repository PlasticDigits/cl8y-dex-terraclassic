import { useEffect, useRef, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getRouteSolveProgress } from '@/services/indexer/client'
import { currentIndicativeAmount, IndexerApiError } from '@/utils/routeQuoteErrors'
import { isIndexerLocalQuoteRejection } from '@/utils/indexerErrors'

/** Watch local publication identity after an authoritative quote outcome, without repeated solves. */
export function useQuoteErrorRecovery(input: {
  error: unknown
  currentIntent: boolean
  tokenIn: string
  tokenOut: string
  amount: string
  exactOutput?: boolean
  trader?: string
  maxMakerFills: number
  maxSpread: string
  poolOnly?: boolean
  refetch: () => unknown
}): string | undefined {
  const {
    error,
    currentIntent,
    tokenIn,
    tokenOut,
    amount,
    exactOutput,
    trader,
    maxMakerFills,
    maxSpread,
    poolOnly,
    refetch,
  } = input
  const rejected = error instanceof IndexerApiError && isIndexerLocalQuoteRejection(error) ? error : undefined
  const [nowMs, setNowMs] = useState(Date.now)
  useEffect(() => {
    const expires = rejected?.indicativeExpiresAtMs
    if (expires == null) return
    setNowMs(Date.now())
    const timer = window.setTimeout(() => setNowMs(Date.now()), Math.max(0, expires - Date.now()))
    return () => window.clearTimeout(timer)
  }, [rejected])

  const progress = useQuery({
    queryKey: [
      'blockedQuoteGeneration',
      tokenIn,
      tokenOut,
      amount,
      exactOutput,
      trader,
      maxMakerFills,
      maxSpread,
      poolOnly,
      rejected?.quoteGeneration,
    ],
    queryFn: ({ signal }) =>
      getRouteSolveProgress(tokenIn, tokenOut, amount, {
        exactOutput,
        trader,
        maxMakerFills,
        maxSpread,
        poolOnly,
        signal,
      }),
    enabled: currentIntent && !!rejected?.quoteGeneration && amount !== '0',
    retry: false,
    refetchOnWindowFocus: false,
    refetchInterval: (query) => (query.state.fetchStatus === 'fetching' ? false : 10_000),
  })
  const attempted = useRef<string | undefined>(undefined)
  useEffect(() => {
    const generation = progress.data?.quote_generation
    if (
      !currentIntent ||
      progress.isFetching ||
      !rejected?.quoteGeneration ||
      typeof generation !== 'string' ||
      !/^[a-zA-Z0-9:_-]{1,256}$/.test(generation) ||
      generation === rejected.quoteGeneration
    )
      return
    const identity = JSON.stringify([
      tokenIn,
      tokenOut,
      amount,
      exactOutput,
      trader,
      maxMakerFills,
      maxSpread,
      poolOnly,
      generation,
    ])
    if (attempted.current === identity) return
    attempted.current = identity
    void refetch()
  }, [
    progress.data,
    progress.isFetching,
    rejected,
    currentIntent,
    tokenIn,
    tokenOut,
    amount,
    exactOutput,
    trader,
    maxMakerFills,
    maxSpread,
    poolOnly,
    refetch,
  ])
  // Compute against actual time on every render too, including intent changes after the timeout.
  return currentIndicativeAmount(error, currentIntent, Math.max(nowMs, Date.now()))
}
