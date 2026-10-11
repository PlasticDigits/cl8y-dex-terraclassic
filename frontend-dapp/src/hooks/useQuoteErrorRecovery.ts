import { useEffect, useRef, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getRouteSolveProgress } from '@/services/indexer/client'
import { currentIndicativeAmount, IndexerApiError } from '@/utils/routeQuoteErrors'
import { isIndexerLocalQuoteRejection, isIndexerPricingDataPending } from '@/utils/indexerErrors'

/** Watch local publication identity after an authoritative quote outcome, without repeated solves. */
export function useQuoteErrorRecovery(input: {
  error: unknown
  currentIntent: boolean
  /** Current UI draft identity; progress request parameters remain submit-aligned. */
  intentKey?: string
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
    intentKey,
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
  const canWatch = !!rejected && (!!rejected.quoteGeneration || isIndexerPricingDataPending(rejected))
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
      intentKey,
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
    enabled: currentIntent && canWatch && amount !== '0',
    retry: false,
    staleTime: 0,
    refetchOnWindowFocus: false,
    refetchInterval: (query) => (query.state.fetchStatus === 'fetching' ? false : 10_000),
  })
  const attempted = useRef<{ intent: string; generations: Set<string> }>({ intent: '', generations: new Set() })
  useEffect(() => {
    const identity =
      intentKey ?? JSON.stringify([tokenIn, tokenOut, amount, exactOutput, trader, maxMakerFills, maxSpread, poolOnly])
    // Reset even for a disabled/successful intermediate intent. A→B→A is
    // explicit new ownership, not a timer-driven retry of the earlier A.
    if (attempted.current.intent !== identity) attempted.current = { intent: identity, generations: new Set() }
    const generation = progress.data?.quote_generation
    if (
      !currentIntent ||
      progress.isFetching ||
      !canWatch ||
      typeof generation !== 'string' ||
      !/^[a-zA-Z0-9:_-]{1,256}$/.test(generation) ||
      generation === rejected?.quoteGeneration
    )
      return
    if (attempted.current.generations.has(generation)) return
    attempted.current.generations.add(generation)
    // Bound indefinitely pending waits to64 recent publications. Repeated
    // current generations remain suppressed; eviction happens on insertion only.
    if (attempted.current.generations.size > 64) {
      attempted.current.generations.delete(attempted.current.generations.values().next().value!)
    }
    void refetch()
  }, [
    progress.data,
    progress.isFetching,
    rejected,
    canWatch,
    currentIntent,
    intentKey,
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
