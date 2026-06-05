import type { IndexerLimitBookPageResponse, IndexerShallowLimitOrder } from '@/types'

/** Default rows per indexer `limit-book` page in the dApp (≤ indexer max 100). */
export const LIMIT_BOOK_UI_PAGE_SIZE = 45

export function limitBookPageQueryKey(pairAddress: string, side: 'bid' | 'ask') {
  return ['limitBookPage', pairAddress, side] as const
}

/**
 * Coerce indexer `limit-book` JSON into a safe page shape. During fresh deploy / indexer sync the
 * wire body may omit `orders`; `flatMap` on `undefined` yields `[undefined]` and crashes render (#327).
 */
export function normalizeLimitBookPageResponse(
  raw: Partial<IndexerLimitBookPageResponse> & Pick<IndexerLimitBookPageResponse, 'side'>
): IndexerLimitBookPageResponse {
  const orders: IndexerShallowLimitOrder[] = Array.isArray(raw.orders)
    ? raw.orders.filter((o): o is IndexerShallowLimitOrder => o != null)
    : []
  return {
    side: raw.side,
    orders,
    has_more: raw.has_more === true,
    next_after_order_id: raw.next_after_order_id ?? null,
  }
}
