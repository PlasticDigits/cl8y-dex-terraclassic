import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getPool } from '@/services/terraclassic/pair'
import type { IndexerPair, IndexerTrade, PairInfo } from '@/types'
import {
  hasResolvableTapeRef,
  pairDecimalsForLimitPriceRef,
  resolveLimitOrderPriceRef,
} from '@/utils/limitOrderPriceReference'

/**
 * Resolves limit-order **reference** token1/token0: prefer indexer tape, else on-chain pool spot (GitLab #166).
 * Exposes loading/error flags for `evaluateLimitOrderPricePlaceGate` while the pool query runs.
 */
export function useLimitOrderPriceRefBundle(params: {
  pairAddr: string
  selectedPair: PairInfo | undefined
  indexerPair: IndexerPair | null | undefined
  latestTrade: IndexerTrade | null | undefined
}) {
  const { pairAddr, selectedPair, indexerPair, latestTrade } = params

  const tapeRefOk = useMemo(() => hasResolvableTapeRef(latestTrade, indexerPair), [latestTrade, indexerPair])

  const decimalsForPoolRef = useMemo(
    () => pairDecimalsForLimitPriceRef(indexerPair ?? null, selectedPair ?? null),
    [indexerPair, selectedPair]
  )

  const poolForLimitPriceRefQuery = useQuery({
    queryKey: ['limitOrderPricePoolRef', pairAddr],
    queryFn: () => getPool(pairAddr),
    enabled: pairAddr.startsWith('terra1') && !!selectedPair && !tapeRefOk && decimalsForPoolRef != null,
    staleTime: 10_000,
    retry: 1,
  })

  const { refToken1PerToken0, refSource } = useMemo(
    () =>
      resolveLimitOrderPriceRef({
        latestTrade: latestTrade ?? null,
        indexerPair: indexerPair ?? null,
        pool: poolForLimitPriceRefQuery.data ?? null,
        pairInfo: selectedPair ?? null,
      }),
    [latestTrade, indexerPair, poolForLimitPriceRefQuery.data, selectedPair]
  )

  const needPoolForLimitRef =
    pairAddr.startsWith('terra1') && !!selectedPair && !tapeRefOk && decimalsForPoolRef != null

  const refResolutionLoading = needPoolForLimitRef && poolForLimitPriceRefQuery.isLoading && refToken1PerToken0 == null

  const refResolutionError = needPoolForLimitRef && poolForLimitPriceRefQuery.isError && refToken1PerToken0 == null

  return {
    refToken1PerToken0,
    refSource,
    refResolutionLoading,
    refResolutionError,
  }
}
