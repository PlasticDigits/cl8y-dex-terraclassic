import { isLimitPriceDirectionInvalid, parsePositivePriceHuman } from '@/utils/limitOrderPriceReference'

/**
 * Client-side checks so resting limits stay **strictly on the maker side** of the current
 * on-chain FIFO book (post-only style). The pair contract still inserts at the walked position;
 * this avoids wasted gas on obviously marketable prices and matches CEX-migrant expectations
 * ([GitLab #152](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/152)).
 *
 * **Invariants**
 *
 * - Book prices are **token1 per token0** (same as the pair / limit form).
 * - **Best bid** = highest bid price = first row from indexer `limit-book?side=bid` (head walk).
 * - **Best ask** = lowest ask price = first row from `side=ask`.
 * - A **bid** crosses if `bid_price >= best_ask` (would immediately lift the ask stack).
 * - An **ask** crosses if `ask_price <= best_bid`.
 * - When the opposite book head is **empty**, ladder / ticket UIs fall back to the indexed tape or
 *   AMM pool **reference** (`refToken1PerToken0`) so BID ladders above market still block when
 *   `best_ask` is missing ([#385](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/385)).
 * - Comparisons use normalized decimal strings (no scientific notation); invalid input yields
 *   `valid: false` so the UI can block submit without guessing.
 */

export type DecimalCompareResult = 'lt' | 'eq' | 'gt'

function splitDecimal(s: string): { neg: boolean; int: string; frac: string } | null {
  const t = s.trim()
  if (!t) return null
  const neg = t.startsWith('-')
  const u = neg ? t.slice(1) : t
  if (!/^\d+(\.\d+)?$/.test(u)) return null
  const [intRaw, fracRaw = ''] = u.split('.')
  const int = intRaw.replace(/^0+(?=\d)/, '') || '0'
  const frac = fracRaw
  return { neg, int, frac }
}

/** Lexicographic compare on normalized unsigned `[int, frac]` parts. */
function cmpUnsignedParts(a: { int: string; frac: string }, b: { int: string; frac: string }): DecimalCompareResult {
  if (a.int.length !== b.int.length) {
    return a.int.length < b.int.length ? 'lt' : 'gt'
  }
  if (a.int !== b.int) {
    return a.int < b.int ? 'lt' : 'gt'
  }
  const maxF = Math.max(a.frac.length, b.frac.length)
  const fa = a.frac.padEnd(maxF, '0')
  const fb = b.frac.padEnd(maxF, '0')
  if (fa === fb) return 'eq'
  return fa < fb ? 'lt' : 'gt'
}

export function comparePositiveDecimalStrings(a: string, b: string): DecimalCompareResult | null {
  const pa = splitDecimal(a)
  const pb = splitDecimal(b)
  if (!pa || !pb || pa.neg || pb.neg) return null
  return cmpUnsignedParts({ int: pa.int, frac: pa.frac }, { int: pb.int, frac: pb.frac })
}

export function limitBidCrossesBestAsk(bidPrice: string, bestAsk: string | null | undefined): boolean | null {
  if (!bestAsk?.trim()) return false
  const c = comparePositiveDecimalStrings(bidPrice, bestAsk)
  if (c == null) return null
  return c === 'gt' || c === 'eq'
}

export function limitAskCrossesBestBid(askPrice: string, bestBid: string | null | undefined): boolean | null {
  if (!bestBid?.trim()) return false
  const c = comparePositiveDecimalStrings(askPrice, bestBid)
  if (c == null) return null
  return c === 'lt' || c === 'eq'
}

export function describeLimitCrossingBlocker(
  side: 'bid' | 'ask',
  price: string,
  bestBid: string | null,
  bestAsk: string | null
): string | null {
  if (side === 'bid') {
    const x = limitBidCrossesBestAsk(price, bestAsk)
    if (x === true) {
      return `Bid price must stay below the best ask (${bestAsk}) so the order does not cross the spread.`
    }
    if (x === null) return 'Enter a valid positive price.'
    return null
  }
  const x = limitAskCrossesBestBid(price, bestBid)
  if (x === true) {
    return `Ask price must stay above the best bid (${bestBid}) so the order does not cross the spread.`
  }
  if (x === null) return 'Enter a valid positive price.'
  return null
}

/**
 * Book-head crossing check with tape / pool reference fallback when the opposite side of the book
 * has no resting orders. Matches {@link TradeOrderTicket} parity: book head first, then reference
 * direction guard ([#385](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/385)).
 */
export function describeLimitCrossingBlockerWithRef(
  side: 'bid' | 'ask',
  price: string,
  bestBid: string | null,
  bestAsk: string | null,
  refToken1PerToken0?: number | null
): string | null {
  const book = describeLimitCrossingBlocker(side, price, bestBid, bestAsk)
  if (book) return book

  const ref = refToken1PerToken0
  if (ref == null || !(ref > 0) || !Number.isFinite(ref)) return null

  const limit = parsePositivePriceHuman(price)
  if (limit == null) return null

  if (side === 'bid' && !bestAsk?.trim() && isLimitPriceDirectionInvalid('bid', limit, ref)) {
    return `Bid price must stay below the market reference (${ref}) so the order does not cross the spread.`
  }
  if (side === 'ask' && !bestBid?.trim() && isLimitPriceDirectionInvalid('ask', limit, ref)) {
    return `Ask price must stay above the market reference (${ref}) so the order does not cross the spread.`
  }
  return null
}
