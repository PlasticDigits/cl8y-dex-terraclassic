import { useQuery } from '@tanstack/react-query'
import { getPairLimitCancellations } from '@/services/indexer/client'

/** Indexed `cancel_limit_order` events for limit UX (duplicate-cancel guard, GitLab #135). */
export function usePairLimitCancellations(pairAddr: string) {
  return useQuery({
    queryKey: ['limitCancellations', pairAddr],
    queryFn: () => getPairLimitCancellations(pairAddr, { limit: 200 }),
    enabled: pairAddr.startsWith('terra1'),
  })
}
