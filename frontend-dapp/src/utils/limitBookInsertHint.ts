import type { IndexerShallowLimitOrder } from '@/types'
import { comparePositiveDecimalStrings } from '@/utils/limitOrderNonCrossing'

export interface LimitBookInsertHintContext {
  /** True when indexer pagination has not reached the book tail for this side. */
  hasMore: boolean
}

/**
 * Resolve the predecessor order id for a new limit placement on one book side.
 * Returns `null` for head insert, unknown neighborhood (pagination gap), or invalid price.
 *
 * Uses loaded deep-book pages only — never guesses across pagination gaps (GitLab #261).
 * Ordering rules: [docs/limit-orders.md § Ordering](../docs/limit-orders.md#ordering-composite-key-fifo).
 */
export function resolveLimitInsertHintAfter(
  side: 'bid' | 'ask',
  priceHuman: string,
  loadedOrders: IndexerShallowLimitOrder[],
  context: LimitBookInsertHintContext
): number | null {
  if (loadedOrders.length === 0) return null

  let prevOrderId: number | null = null

  for (const order of loadedOrders) {
    const cmp = comparePositiveDecimalStrings(priceHuman.trim(), order.price.trim())
    if (cmp == null) return null

    if (side === 'bid') {
      if (cmp === 'gt') return prevOrderId
      if (cmp === 'eq') {
        prevOrderId = order.order_id
        continue
      }
      prevOrderId = order.order_id
      continue
    }

    if (cmp === 'lt') return prevOrderId
    if (cmp === 'eq') {
      prevOrderId = order.order_id
      continue
    }
    prevOrderId = order.order_id
  }

  if (context.hasMore) return null
  return prevOrderId
}

/** Flatten infinite-query pages into one head-to-tail book slice for hint resolution. */
export function flattenLimitBookPages(pages: { orders: IndexerShallowLimitOrder[]; has_more: boolean }[] | undefined): {
  orders: IndexerShallowLimitOrder[]
  hasMore: boolean
} {
  if (!pages?.length) return { orders: [], hasMore: false }
  const orders = pages.flatMap((p) => p.orders ?? [])
  const last = pages[pages.length - 1]
  return { orders, hasMore: last?.has_more ?? false }
}
