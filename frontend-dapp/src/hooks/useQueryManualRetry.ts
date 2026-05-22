import { useCallback } from 'react'
import { useQueryClient, type QueryKey, type UseQueryResult } from '@tanstack/react-query'

/**
 * Manual retry for failed React Query reads with a high `staleTime`.
 * `refetch()` alone can appear to no-op while the cache entry is still "fresh";
 * invalidate + refetch guarantees a new network round-trip ([GitLab #177](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/177)).
 */
export function useQueryManualRetry<TData>(
  queryKey: QueryKey,
  query: Pick<UseQueryResult<TData>, 'refetch' | 'isFetching'>
) {
  const queryClient = useQueryClient()

  const retry = useCallback(() => {
    void queryClient.invalidateQueries({ queryKey, exact: true })
    void query.refetch({ cancelRefetch: false })
  }, [query, queryClient, queryKey])

  return {
    retry,
    isRetrying: query.isFetching,
  }
}
