import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toastErrorMessage, useOptionalToast } from '@/contexts/toastContextState'
import { cancelLimitOrder, cancelLimitOrders } from '@/services/terraclassic/pair'
import { sounds } from '@/lib/sounds'
import type { IndexerLimitCancellation, PairOrderStatusKind } from '@/types'
import { orderIdHasIndexedCancellation } from '@/utils/limitOrderCancelUserMessage'
import { limitOrderStatusQueryKey, recentlyCancelledOrderIdsQueryKey } from '@/hooks/useLimitOrderStatuses'

export type LimitOrderCancelInput = number | number[]

function normalizeCancelOrderIds(input: LimitOrderCancelInput): number[] {
  const ids = (Array.isArray(input) ? input : [input]).filter((id) => Number.isFinite(id) && id >= 1)
  if (ids.length === 0) {
    throw new Error('Invalid order id')
  }
  return ids
}

function rememberCancelledIds(queryClient: ReturnType<typeof useQueryClient>, pairAddr: string, orderIds: number[]) {
  queryClient.setQueryData<number[]>(recentlyCancelledOrderIdsQueryKey(pairAddr), (old) => [
    ...new Set([...(old ?? []), ...orderIds]),
  ])
  for (const orderId of orderIds) {
    queryClient.setQueryData<PairOrderStatusKind>(limitOrderStatusQueryKey(pairAddr, orderId), 'unknown')
  }
}

/**
 * On-chain cancel for a pair — single order or batch (GitLab #246). Shared by the trade ticket
 * and order-book row actions ([GitLab #162](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/162)).
 * Preflight skips LCD `Unknown` / `ParkedRefund` and indexed cancels so a stale ● row cannot grief gas (#530).
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
      const recent = queryClient.getQueryData<number[]>(recentlyCancelledOrderIdsQueryKey(pairAddr)) ?? []
      for (const orderId of orderIds) {
        if (orderIdHasIndexedCancellation(cancels, orderId) || recent.includes(orderId)) {
          throw new Error(`Order #${orderId} has already been cancelled.`)
        }
        const lcd = queryClient.getQueryData<PairOrderStatusKind | undefined>(
          limitOrderStatusQueryKey(pairAddr, orderId)
        )
        if (lcd === 'unknown') {
          throw new Error(
            'This limit order is no longer on the book. It may have already been cancelled or fully filled.'
          )
        }
        if (lcd === 'parked_refund') {
          throw new Error('This order is parked. Use Claim refund instead of Cancel.')
        }
      }
      if (orderIds.length === 1) {
        return cancelLimitOrder(walletAddress, pairAddr, orderIds[0]!)
      }
      return cancelLimitOrders(walletAddress, pairAddr, orderIds)
    },
    onSuccess: (_data, variables) => {
      const orderIds = normalizeCancelOrderIds(variables)
      rememberCancelledIds(queryClient, pairAddr, orderIds)
      sounds.playSuccess()
      toastApi?.pushToast('success', 'Limit order cancelled.')
      queryClient.invalidateQueries({ queryKey: ['limitPlacements'] })
      queryClient.invalidateQueries({ queryKey: ['limitCancellations', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['limitFills'] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPage', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPagePreview', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['tradeBestBook', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['wallet-indexer-history'] })
      queryClient.invalidateQueries({ queryKey: ['limitOrderStatus', pairAddr] })
    },
    onError: (error) => {
      sounds.playError()
      toastApi?.pushToast('error', toastErrorMessage(error))
    },
  })
}
