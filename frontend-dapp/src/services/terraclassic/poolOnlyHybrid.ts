import type { HybridSwapParams } from '@/types'

/** Pool-only quote leg: entire offer on reserves (`book_input = 0`). See invariant L8 / GitLab #190. */
export function poolOnlyHybridParams(offerAmount: string): HybridSwapParams {
  return {
    pool_input: offerAmount,
    book_input: '0',
    max_maker_fills: 1,
    book_start_hint: null,
  }
}

/** Ratio template for `hybrid_reverse_simulation` when the offer is pool-only. */
export function poolOnlyHybridTemplate(): HybridSwapParams {
  return {
    pool_input: '1',
    book_input: '0',
    max_maker_fills: 1,
    book_start_hint: null,
  }
}
