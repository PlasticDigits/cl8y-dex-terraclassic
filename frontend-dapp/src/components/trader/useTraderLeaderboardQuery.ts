import { useQuery } from '@tanstack/react-query'
import { getLeaderboard } from '@/services/indexer/client'
import { TRADER_LEADERBOARD_LIMIT, TRADER_LEADERBOARD_REFETCH_MS, type LeaderboardSort } from './traderLeaderboard'

function pairQueryKey(pairAddress?: string): string {
  return pairAddress?.trim() || 'global'
}

/**
 * Shared React Query for global (#657) and pair-scoped (#666) boards.
 * Key includes pair (`CS-14`). Charts outage banner observes the default-sort query.
 */
export function useTraderLeaderboardQuery(sort: LeaderboardSort, pairAddress: string | undefined, enabled: boolean) {
  const pairTrim = pairAddress?.trim() || ''
  return useQuery({
    queryKey: ['leaderboard', sort, pairQueryKey(pairAddress)],
    queryFn: () =>
      pairTrim
        ? getLeaderboard(sort, TRADER_LEADERBOARD_LIMIT, pairTrim)
        : getLeaderboard(sort, TRADER_LEADERBOARD_LIMIT),
    enabled: enabled && (!pairAddress || !!pairTrim),
    refetchInterval: TRADER_LEADERBOARD_REFETCH_MS,
  })
}
