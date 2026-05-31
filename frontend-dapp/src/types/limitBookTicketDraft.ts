/** Prefill payload from the trade order book “Edit” row action (GitLab #162, #178, #247). */
export type LimitBookTicketDraft = {
  side: 'bid' | 'ask'
  price: string
  amountHuman: string
  /** Resting order being edited — enables price-only `UpdateLimitOrderPrice` when other fields unchanged. */
  orderId: number
  expiresAt?: number | null
  /** Advisory book-walk hint for on-chain relink (indexer row order). */
  hintAfterOrderId?: number | null
}

/** Snapshot of ticket fields when Edit prefill is applied — detects price-only amend vs replace. */
export type LimitBookEditContext = {
  orderId: number
  side: 'bid' | 'ask'
  price: string
  amountHuman: string
  expiresAt: number | null
}
