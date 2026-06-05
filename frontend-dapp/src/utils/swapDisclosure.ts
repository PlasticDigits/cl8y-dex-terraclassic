import { tokenAssetInfo, type IndexerRouteQuoteKind } from '@/types'
import { isDecimalAmountDraft, tryParseBigInt } from '@/utils/decimalAmountInput'
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
  const bookHuman = input.bookInputHuman.trim()
  if (bookHuman && !isDecimalAmountDraft(bookHuman)) {
    return null
  }
  const bookRaw = bookHuman ? toRawAmount(bookHuman, dec) : '0'
  const total = tryParseBigInt(input.rawInputAmount || '0')
  const book = tryParseBigInt(bookRaw)
  if (total === null || book === null) return null
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
  if (kind === 'indexer_hybrid_lcd' || kind === 'indexer_hybrid_db') {
    return {
      show: true,
      title: 'Indexer hybrid',
      line:
        kind === 'indexer_hybrid_db'
          ? 'Route uses pool + limit book legs priced from the indexer’s Postgres mirror; final amount validated with router `simulate_swap_operations` when configured.'
          : 'Route uses pool + limit book legs; quote is your wallet’s LCD `simulate_swap_operations` (matches submit shape on success).',
      degraded: false,
    }
  }
  if (kind === 'indexer_hybrid_lcd_degraded' || kind === 'indexer_hybrid_db_degraded') {
    return {
      show: true,
      title: 'Indexer hybrid',
      line:
        kind === 'indexer_hybrid_db_degraded'
          ? 'Mirror/grid was degraded or router sim disagreed with the indexed quote — treat output as conservative.'
          : 'At least one hop was pool-only on the indexer; remaining legs may still use the book per hop.',
      degraded: true,
    }
  }
  return { show: false }
}
