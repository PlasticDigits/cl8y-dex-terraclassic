import { tokenAssetInfo, type IndexerRouteQuoteKind } from '@/types'
import { fromRawAmount, getDecimals, toRawAmount } from '@/utils/formatAmount'

/** Same pool/book split as `SwapPage` / `swap` mutation for direct CW20 + Settings “limit book leg”. */
export type DirectHybridBookSplit = {
  totalRaw: string
  poolRaw: string
  bookRaw: string
  poolHuman: string
  bookHuman: string
  /** Submit path will include `hybrid` (positive book leg and `max_maker_fills` valid). */
  willSubmitHybrid: boolean
  /** `book_input` would exceed `amount` — do not use split for display as execution intent; mutation errors. */
  bookExceedsPay: boolean
}

/**
 * Pure helper: pool vs book input split for direct CW20 swaps when the limit-book leg is enabled in Settings.
 * Returns `null` when the hybrid book UI does not apply (non-direct, feature off, or non-CW20 pay token).
 */
export function getDirectHybridBookSplit(input: {
  isDirect: boolean
  useHybridBook: boolean
  fromToken: string
  bookInputHuman: string
  rawInputAmount: string
  hybridMaxMakers: number
}): DirectHybridBookSplit | null {
  if (!input.isDirect || !input.useHybridBook || !input.fromToken.startsWith('terra1')) {
    return null
  }
  const pay = tokenAssetInfo(input.fromToken)
  const dec = getDecimals(pay)
  const bookRaw = input.bookInputHuman.trim() ? toRawAmount(input.bookInputHuman.trim(), dec) : '0'
  const total = BigInt(input.rawInputAmount || '0')
  const book = BigInt(bookRaw)
  if (book > total) {
    return {
      totalRaw: input.rawInputAmount,
      poolRaw: '0',
      bookRaw: bookRaw,
      poolHuman: fromRawAmount('0', dec),
      bookHuman: fromRawAmount(bookRaw, dec),
      willSubmitHybrid: false,
      bookExceedsPay: true,
    }
  }
  const pool = total - book
  const canSubmit = book > 0n && input.hybridMaxMakers >= 1
  return {
    totalRaw: input.rawInputAmount,
    poolRaw: pool.toString(),
    bookRaw: book.toString(),
    poolHuman: fromRawAmount(pool.toString(), dec),
    bookHuman: fromRawAmount(book.toString(), dec),
    willSubmitHybrid: canSubmit,
    bookExceedsPay: false,
  }
}

export type IndexerHybridExecution = {
  show: true
  title: 'Indexer hybrid'
  line: string
  degraded: boolean
}

export function getIndexerHybridExecutionSummary(
  kind: IndexerRouteQuoteKind | undefined
): IndexerHybridExecution | { show: false } {
  if (kind === 'indexer_hybrid_lcd') {
    return {
      show: true,
      title: 'Indexer hybrid',
      line: 'Route uses pool + limit book legs; quote is your wallet’s LCD `simulate_swap_operations` (matches submit shape on success).',
      degraded: false,
    }
  }
  if (kind === 'indexer_hybrid_lcd_degraded') {
    return {
      show: true,
      title: 'Indexer hybrid',
      line: 'At least one hop was pool-only on the indexer; remaining legs may still use the book per hop.',
      degraded: true,
    }
  }
  return { show: false }
}
