import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toastErrorMessage, useOptionalToast } from '@/contexts/toastContextState'
import { cancelLimitOrder, cancelLimitOrders } from '@/services/terraclassic/pair'
import { sounds } from '@/lib/sounds'
import type { IndexerLimitCancellation } from '@/types'
import { orderIdHasIndexedCancellation } from '@/utils/limitOrderCancelUserMessage'

export type LimitOrderCancelInput = number | number[]

function normalizeCancelOrderIds(input: LimitOrderCancelInput): number[] {
  const ids = (Array.isArray(input) ? input : [input]).filter((id) => Number.isFinite(id) && id >= 1)
  if (ids.length === 0) {
    throw new Error('Invalid order id')
  }
  return ids
}

/**
 * On-chain cancel for a pair — single order or batch (GitLab #246). Shared by the trade ticket
 * and order-book row actions ([GitLab #162](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/162)).
 */
export function useLimitOrderCancelMutation(pairAddr: string, walletAddress: string | undefined) {
  const queryClient = useQueryClient()
  const toastApi = useOptionalToast()

  return useMutation({
    mutationFn: async (orderIdOrIds: LimitOrderCancelInput) => {
      if (!walletAddress) throw new Error('Connect wallet')
      if (!pairAddr.startsWith('terra1')) throw new Error('Select a pair')
      const orderIds = normalizeCancelOrderIds(orderIdOrIds)
      const cancels = queryClient.getQueryData<IndexerLimitCancellation[]>(['limitCancellations', pairAddr]) ?? []
      for (const orderId of orderIds) {
        if (orderIdHasIndexedCancellation(cancels, orderId)) {
          throw new Error(`Order #${orderId} has already been cancelled.`)
        }
      }
      if (orderIds.length === 1) {
        return cancelLimitOrder(walletAddress, pairAddr, orderIds[0]!)
      }
      return cancelLimitOrders(walletAddress, pairAddr, orderIds)
    },
    onSuccess: () => {
      sounds.playSuccess()
      toastApi?.pushToast('success', 'Limit order cancelled.')
      queryClient.invalidateQueries({ queryKey: ['limitPlacements'] })
      queryClient.invalidateQueries({ queryKey: ['limitCancellations', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPage', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPagePreview', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['tradeBestBook', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['wallet-indexer-history'] })
    },
    onError: (error) => {
      sounds.playError()
      toastApi?.pushToast('error', toastErrorMessage(error))
    },
  })
}
