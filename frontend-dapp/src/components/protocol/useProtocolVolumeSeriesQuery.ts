import { useQuery } from '@tanstack/react-query'
import { getProtocolVolumeSeries } from '@/services/indexer/client'
import type { ProtocolVolumeGrain } from '@/utils/protocolVolumeGrain'
import { isProtocolVolumeDailyUnavailable } from './useProtocolVolumeDailyQuery'

export function useProtocolVolumeSeriesQuery(grain: ProtocolVolumeGrain, limit: number) {
  return useQuery({
    queryKey: ['indexer-protocol-volume-series', grain, limit],
    queryFn: () => getProtocolVolumeSeries(grain, limit),
    refetchInterval: 60_000,
    retry: false,
  })
}

export { isProtocolVolumeDailyUnavailable as isProtocolVolumeSeriesUnavailable }
