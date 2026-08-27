import { useQuery } from '@tanstack/react-query'
import { getProtocolFeesSeries, getProtocolLiquiditySeries, getProtocolVolumeSeries } from '@/services/indexer/client'
import type { ProtocolUtcMetric, ProtocolVolumeGrain } from '@/utils/protocolVolumeGrain'
import { isProtocolVolumeDailyUnavailable } from './useProtocolVolumeDailyQuery'

export function useProtocolUtcSeriesQuery(metric: ProtocolUtcMetric, grain: ProtocolVolumeGrain, limit: number) {
  return useQuery({
    queryKey: ['indexer-protocol-utc-series', metric, grain, limit],
    queryFn: () => {
      if (metric === 'liquidity') return getProtocolLiquiditySeries(grain, limit)
      if (metric === 'fees') return getProtocolFeesSeries(grain, limit)
      return getProtocolVolumeSeries(grain, limit)
    },
    refetchInterval: 60_000,
    retry: false,
  })
}

export function useProtocolVolumeSeriesQuery(grain: ProtocolVolumeGrain, limit: number) {
  return useProtocolUtcSeriesQuery('volume', grain, limit)
}

export { isProtocolVolumeDailyUnavailable as isProtocolVolumeSeriesUnavailable }
