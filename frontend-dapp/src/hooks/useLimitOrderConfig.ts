import { useQuery } from '@tanstack/react-query'

import { getLimitOrderConfig } from '@/services/terraclassic/pair'

const DEFAULT_MAX_BATCH_RUNGS = 20

export function useLimitOrderConfig(pairAddress: string | undefined) {
  return useQuery({
    queryKey: ['limitOrderConfig', pairAddress],
    queryFn: () => getLimitOrderConfig(pairAddress!),
    enabled: Boolean(pairAddress),
    staleTime: 60_000,
    placeholderData: { max_batch_rungs: DEFAULT_MAX_BATCH_RUNGS },
  })
}
