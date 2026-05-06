/**
 * CosmWasm / wasmd often surfaces missing map entries as `type: ...::LimitOrder; key: [...] not found`.
 * GitLab #135 — replace with copy users can act on.
 */
export function humanizeCosmwasmLimitOrderMissingMessage(raw: string): string | null {
  if (!raw.includes('not found')) return null
  const compact = raw.replace(/\s+/g, ' ')
  if (
    raw.includes('LimitOrder') ||
    compact.includes('cl8y_dex_pair::state::LimitOrder') ||
    /limit_orders?/i.test(raw)
  ) {
    return 'This limit order is no longer on the book. It may have already been cancelled or fully filled.'
  }
  return null
}

/** Any indexed cancel for this pair + order id (order no longer cancelable via normal path). */
export function orderIdHasIndexedCancellation(cancellations: Array<{ order_id: number }>, orderId: number): boolean {
  return cancellations.some((c) => c.order_id === orderId)
}
