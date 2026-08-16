import { useQueries, useQuery } from '@tanstack/react-query'
import { parsePairOrderStatus, queryOrderStatus } from '@/services/terraclassic/pair'
import type { PairOrderStatusKind } from '@/types'

export const limitOrderStatusQueryKey = (pairAddr: string, orderId: number) =>
  ['limitOrderStatus', pairAddr, orderId] as const

export const recentlyCancelledOrderIdsQueryKey = (pairAddr: string) =>
  ['limitOrderRecentlyCancelled', pairAddr] as const

/**
 * LCD `OrderStatus` per resting id. A failed query stays `undefined` — never `unknown` (L21 / #530).
 */
export function useLimitOrderStatuses(pairAddr: string, orderIds: number[]) {
  const uniqueIds = [...new Set(orderIds.filter((id) => Number.isFinite(id) && id >= 1))]
  const queries = useQueries({
    queries: uniqueIds.map((orderId) => ({
      queryKey: limitOrderStatusQueryKey(pairAddr, orderId),
      queryFn: async (): Promise<PairOrderStatusKind | null> => {
        const raw = await queryOrderStatus(pairAddr, orderId)
        return parsePairOrderStatus(raw) ?? null
      },
      enabled: pairAddr.startsWith('terra1'),
      staleTime: 10_000,
      refetchInterval: 15_000,
      retry: false,
    })),
  })

  const byOrderId: Record<number, PairOrderStatusKind | undefined> = {}
  uniqueIds.forEach((id, i) => {
    const q = queries[i]
    if (q?.isError || q?.data == null) {
      byOrderId[id] = undefined
      return
    }
    byOrderId[id] = q.data
  })

  return { byOrderId, queries }
}

/** Optimistic cancel ids for indexer lag (I9). Seeded by `useLimitOrderCancelMutation`. */
export function useRecentlyCancelledOrderIds(pairAddr: string): number[] {
  const q = useQuery({
    queryKey: recentlyCancelledOrderIdsQueryKey(pairAddr),
    queryFn: async () => [] as number[],
    enabled: pairAddr.startsWith('terra1'),
    staleTime: Infinity,
    gcTime: 30 * 60_000,
    initialData: [],
  })
  return q.data ?? []
}
