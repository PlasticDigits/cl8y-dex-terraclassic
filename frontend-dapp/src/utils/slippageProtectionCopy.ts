/** Retail label for user-configured on-chain `max_spread` guard (GitLab #412). */
export const SLIPPAGE_PROTECTION_LABEL = 'Slippage protection'

/** Footnote in Settings — on-chain parameter name. */
export const SLIPPAGE_PROTECTION_ON_CHAIN_FOOTNOTE = 'Sent on-chain as max_spread (price impact cap per hop).'

/** Route execution-quality metric (#293) — distinct from tolerance. */
export const ROUTE_EXECUTION_SLIPPAGE_LABEL = 'Expected slippage'

export const ROUTE_EXECUTION_SLIPPAGE_TOOLTIP =
  'Execution-quality estimate vs fair cross-rate token prices. This is not your slippage protection setting in Settings.'

export const TRANSACTION_DEADLINE_LABEL = 'Transaction deadline'

/** Format seconds for retail deadline display (e.g. 300 → "5 min"). */
export function formatTransactionDeadline(seconds: number): string {
  if (seconds % 60 === 0 && seconds >= 60) {
    const minutes = seconds / 60
    return minutes === 1 ? '1 min' : `${minutes} min`
  }
  return seconds === 1 ? '1 sec' : `${seconds} sec`
}
