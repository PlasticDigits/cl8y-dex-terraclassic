import { useMutation, useQueryClient } from '@tanstack/react-query'
import { updateLimitOrderPrice } from '@/services/terraclassic/pair'
import { sounds } from '@/lib/sounds'

export type LimitOrderUpdatePriceInput = {
  orderId: number
  price: string
  maxAdjustSteps: number
  hintAfterOrderId?: number | null
}

/**
 * Owner-only in-place limit price relink (`ExecuteMsg::UpdateLimitOrderPrice`, GitLab #247).
 */
export function useLimitOrderUpdatePriceMutation(pairAddr: string, walletAddress: string | undefined) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ orderId, price, maxAdjustSteps, hintAfterOrderId }: LimitOrderUpdatePriceInput) => {
      if (!walletAddress) throw new Error('Connect wallet')
      if (!pairAddr.startsWith('terra1')) throw new Error('Select a pair')
      if (!Number.isFinite(orderId) || orderId < 1) throw new Error('Invalid order id')
      return updateLimitOrderPrice(walletAddress, pairAddr, orderId, price, maxAdjustSteps, hintAfterOrderId)
    },
    onSuccess: () => {
      sounds.playSuccess()
      queryClient.invalidateQueries({ queryKey: ['limitPlacements'] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPage', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPagePreview', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['tradeBestBook', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['wallet-indexer-history'] })
    },
    onError: () => sounds.playError(),
  })
}
