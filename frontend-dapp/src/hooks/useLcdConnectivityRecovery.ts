import { useEffect, useRef } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { LCD_CONNECTIVITY_RECOVERY_POLL_MS, probeLcdReachability } from '@/utils/lcdConnectivity'

export const LCD_HEALTH_QUERY_KEY = ['lcd-health-probe'] as const

/**
 * Polls LCD reachability and invalidates React Query caches when the node recovers
 * so traders are not stuck behind a failed load until a tab switch ([GitLab #171](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/171)).
 */
export function useLcdConnectivityRecovery() {
  const queryClient = useQueryClient()
  const wasReachableRef = useRef<boolean | null>(null)

  const healthQuery = useQuery({
    queryKey: LCD_HEALTH_QUERY_KEY,
    queryFn: probeLcdReachability,
    staleTime: 0,
    refetchInterval: LCD_CONNECTIVITY_RECOVERY_POLL_MS,
    refetchIntervalInBackground: true,
    retry: false,
  })

  const isLcdReachable = healthQuery.data === true
  const isLcdUnreachable = healthQuery.data === false
  const isProbePending = healthQuery.isPending || healthQuery.isFetching

  useEffect(() => {
    if (healthQuery.data === undefined) return
    const prev = wasReachableRef.current
    wasReachableRef.current = healthQuery.data
    if (prev === false && healthQuery.data === true) {
      void queryClient.invalidateQueries()
    }
  }, [healthQuery.data, queryClient])

  const retryAll = () => {
    void healthQuery.refetch()
    void queryClient.invalidateQueries()
  }

  return {
    isLcdReachable,
    isLcdUnreachable,
    isProbePending,
    retryAll,
    healthQuery,
  }
}
