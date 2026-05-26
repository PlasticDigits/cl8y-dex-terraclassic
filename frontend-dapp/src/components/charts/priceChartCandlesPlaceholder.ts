import type { Query } from '@tanstack/react-query'

/** React Query key shape for `PriceChart` candles: `['candles', pairAddress, interval]`. */
export type CandlesQueryKey = readonly ['candles', string, string]

/**
 * Keep prior candle rows only when the **pair** is unchanged (interval switch).
 * On pair switch, return `undefined` so the plot unmounts and stale OHLC is not shown
 * (GitLab #148 interval stability + #180 / pair-switch invariants).
 */
export function keepPreviousCandlesForIntervalSwitch<T>(
  pairAddress: string,
  previousData: T | undefined,
  previousQuery: Pick<Query, 'queryKey'> | undefined
): T | undefined {
  const prevKey = previousQuery?.queryKey as CandlesQueryKey | undefined
  if (prevKey?.[0] === 'candles' && prevKey[1] === pairAddress) {
    return previousData
  }
  return undefined
}
