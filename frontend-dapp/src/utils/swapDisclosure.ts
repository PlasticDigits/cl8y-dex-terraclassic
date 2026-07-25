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

/**
 * Retail Execution block for Swap Settings hybrid split (GitLab #492).
 * Silence when hybrid Settings are on but the manual book leg is empty — do not tell
 * users to “add a book leg” (feature already enabled; pool-only is expected).
 */
export type DirectHybridSettingsExecutionSummary =
  | { show: false }
  | {
      show: true
      variant: 'book_exceeds_pay' | 'hybrid_manual_split' | 'max_makers_blocked'
      tone: 'negative' | 'neutral' | 'warning'
      /** Short retail line; hybrid_manual_split uses pool/book amounts instead. */
      line?: string
      poolHuman?: string
      bookHuman?: string
      poolRaw?: string
      bookRaw?: string
    }

/**
 * Display helper for the Swap Settings → Execution hybrid split block.
 *
 * Invariants (#492, cognitive load):
 * - Hybrid Settings on + empty manual book (`bookRaw === 0`, `!willSubmitHybrid`) → `{ show: false }`
 *   (no “Pool only — add a book leg…” copy; silence over instructional fluff).
 * - Keep short warnings only for actionable errors (book > pay, max makers &lt; 1).
 */
export function getDirectHybridSettingsExecutionSummary(
  split: DirectHybridBookSplit | null
): DirectHybridSettingsExecutionSummary {
  if (!split) return { show: false }
  if (split.bookExceedsPay) {
    return {
      show: true,
      variant: 'book_exceeds_pay',
      tone: 'negative',
      line: 'Book leg is larger than your pay amount.',
    }
  }
  if (split.willSubmitHybrid) {
    return {
      show: true,
      variant: 'hybrid_manual_split',
      tone: 'neutral',
      poolHuman: split.poolHuman,
      bookHuman: split.bookHuman,
      poolRaw: split.poolRaw,
      bookRaw: split.bookRaw,
    }
  }
  const book = tryParseBigInt(split.bookRaw)
  if (book !== null && book > 0n) {
    return {
      show: true,
      variant: 'max_makers_blocked',
      tone: 'warning',
      line: 'Set max distinct makers to at least 1.',
    }
  }
  // Hybrid on, empty manual book → pool-only path; no Execution notice (#492).
  return { show: false }
}

export type IndexerHybridExecution = {
  show: true
  title: 'Limit book + pool'
  line: string
  degraded: boolean
}

export function getIndexerHybridExecutionSummary(
  kind: IndexerRouteQuoteKind | undefined
): IndexerHybridExecution | { show: false } {
  if (kind === 'indexer_hybrid_lcd' || kind === 'indexer_hybrid_db') {
    return {
      show: true,
      title: 'Limit book + pool',
      line:
        kind === 'indexer_hybrid_db'
          ? 'Your trade may fill against resting limit orders and the pool. The estimate uses indexed market data and is checked before you submit.'
          : 'Your trade may fill against resting limit orders and the pool. The estimate is checked against your wallet before you submit.',
      degraded: false,
    }
  }
  if (kind === 'indexer_hybrid_lcd_degraded' || kind === 'indexer_hybrid_db_degraded') {
    return {
      show: true,
      title: 'Limit book + pool',
      line:
        kind === 'indexer_hybrid_db_degraded'
          ? 'Market data was partially unavailable — treat the estimated output as conservative.'
          : 'One or more hops used pool-only pricing; other legs may still use the limit book.',
      degraded: true,
    }
  }
  return { show: false }
}
