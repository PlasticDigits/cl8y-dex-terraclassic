import { useQuery } from '@tanstack/react-query'
import { getProtocolVolumeDaily } from '@/services/indexer/client'

export function useProtocolVolumeDailyQuery(days: 7 | 30) {
  return useQuery({
    queryKey: ['indexer-protocol-volume-daily', days],
    queryFn: () => getProtocolVolumeDaily(days),
    refetchInterval: 60_000,
    retry: false,
  })
}

export function isProtocolVolumeDailyUnavailable(err: unknown): boolean {
  if (!(err instanceof Error)) return false
  return /Indexer API error: 404/.test(err.message) || /Indexer API error: 501/.test(err.message)
}
