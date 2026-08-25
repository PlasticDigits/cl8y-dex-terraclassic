import { useQuery } from '@tanstack/react-query'
import {
  loadUnregisteredFactoryPairs,
  pickHighestLpUnregistered,
  tokensNeedingRegisterForManager,
} from '@/utils/communityTaxRegisterPair'

export function useManageListedPairCatchUp(input: {
  token: string
  managerWallet: boolean
  taxTemplate: boolean
  connected: string | null
}) {
  const enabled = Boolean(input.managerWallet && input.taxTemplate && input.token)
  const unregQuery = useQuery({
    queryKey: ['communityTaxUnregisteredPairs', input.token],
    queryFn: () => loadUnregisteredFactoryPairs(input.token),
    enabled,
    staleTime: 15_000,
  })
  const otherQuery = useQuery({
    queryKey: ['communityTaxOtherUnregistered', input.connected, input.token],
    queryFn: () => tokensNeedingRegisterForManager(input.connected!, input.token),
    enabled: enabled && Boolean(input.connected),
    staleTime: 30_000,
  })
  const unregistered = unregQuery.data ?? []
  const target = pickHighestLpUnregistered(unregistered)
  const show = enabled && unregistered.length > 0 && Boolean(target)
  return {
    show,
    target,
    leftover: Math.max(0, unregistered.length - 1),
    otherTokens: otherQuery.data ?? [],
    refetch: async () => {
      await unregQuery.refetch()
      await otherQuery.refetch()
    },
  }
}
