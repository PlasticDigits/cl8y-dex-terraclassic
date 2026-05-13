import { useMutation, useQueryClient } from '@tanstack/react-query'
import { cancelLimitOrder } from '@/services/terraclassic/pair'
import { sounds } from '@/lib/sounds'
import type { IndexerLimitCancellation } from '@/types'
import { orderIdHasIndexedCancellation } from '@/utils/limitOrderCancelUserMessage'

/**
 * On-chain `cancel_limit_order` for a pair. Shared by the trade ticket and order-book row actions
 * so one mutation drives loading / invalidations ([GitLab #162](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/162)).
 */
export function useLimitOrderCancelMutation(pairAddr: string, walletAddress: string | undefined) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (orderId: number) => {
      if (!walletAddress) throw new Error('Connect wallet')
      if (!pairAddr.startsWith('terra1')) throw new Error('Select a pair')
      if (!Number.isFinite(orderId) || orderId < 1) throw new Error('Invalid order id')
      const cancels = queryClient.getQueryData<IndexerLimitCancellation[]>(['limitCancellations', pairAddr]) ?? []
      if (orderIdHasIndexedCancellation(cancels, orderId)) {
        throw new Error('This order has already been cancelled.')
      }
      return cancelLimitOrder(walletAddress, pairAddr, orderId)
    },
    onSuccess: () => {
      sounds.playSuccess()
      queryClient.invalidateQueries({ queryKey: ['limitPlacements'] })
      queryClient.invalidateQueries({ queryKey: ['limitCancellations', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPage', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPagePreview', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['tradeBestBook', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['wallet-indexer-history'] })
    },
    onError: () => sounds.playError(),
  })
}
