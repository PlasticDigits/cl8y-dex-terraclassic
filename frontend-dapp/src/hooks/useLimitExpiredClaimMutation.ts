import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toastErrorMessage, useOptionalToast } from '@/contexts/toastContextState'
import { claimExpiredLimitOrder, claimExpiredLimitOrders } from '@/services/terraclassic/pair'
import { sounds } from '@/lib/sounds'
import { normalizeExpiredClaimOrderIds, type LimitExpiredClaimInput } from '@/utils/limitExpiredClaimBatch'

export type { LimitExpiredClaimInput }

/**
 * On-chain parked-expired refund for a pair — single order or batch (GitLab #246, #253).
 * Shared by {@link LimitOrderMyPlacementsPanel} on `/limits` and `/trade`.
 */
export function useLimitExpiredClaimMutation(pairAddr: string, walletAddress: string | undefined) {
  const queryClient = useQueryClient()
  const toastApi = useOptionalToast()

  return useMutation({
    mutationFn: async (orderIdOrIds: LimitExpiredClaimInput) => {
      if (!walletAddress) throw new Error('Connect wallet')
      if (!pairAddr.startsWith('terra1')) throw new Error('Select a pair')
      const orderIds = normalizeExpiredClaimOrderIds(orderIdOrIds)
      if (orderIds.length === 1) {
        return claimExpiredLimitOrder(walletAddress, pairAddr, orderIds[0]!)
      }
      return claimExpiredLimitOrders(walletAddress, pairAddr, orderIds)
    },
    onSuccess: () => {
      sounds.playSuccess()
      toastApi?.pushToast('success', 'Expired limit order claimed.')
      queryClient.invalidateQueries({ queryKey: ['limitPlacements'] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPagePreview', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPage', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
    },
    onError: (error) => {
      sounds.playError()
      toastApi?.pushToast('error', toastErrorMessage(error))
    },
  })
}
