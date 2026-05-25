import { shortenAddress } from '@/utils/tokenDisplay'
import {
  TRADE_UNKNOWN_PAIR_LINK_CTA,
  TRADE_UNKNOWN_PAIR_LINK_LEAD,
  TRADE_UNKNOWN_PAIR_LINK_TITLE,
} from '@/utils/tradeUnknownPairLinkCopy'

export interface PairNotFoundLinkNoticeProps {
  /** Decoded route segment (shown quoted, truncated when long). */
  unknownParam: string
  pairSelectId: string
  onDismiss?: () => void
  className?: string
}

function formatUnknownParamDisplay(raw: string): string {
  const trimmed = raw.trim()
  if (trimmed.length <= 48) return trimmed
  return shortenAddress(trimmed, 10, 8)
}

export function PairNotFoundLinkNotice({
  unknownParam,
  pairSelectId,
  onDismiss,
  className = '',
}: PairNotFoundLinkNoticeProps) {
  const display = formatUnknownParamDisplay(unknownParam)

  const focusPairSelect = () => {
    const el = document.getElementById(pairSelectId)
    if (!el || !(el instanceof HTMLElement)) return
    el.scrollIntoView?.({ block: 'nearest', behavior: 'smooth' })
    el.focus()
    el.click()
  }

  return (
    <div
      className={`alert-warning text-sm ${className}`.trim()}
      role="alert"
      data-testid="trade-pair-not-found-link-notice"
    >
      <p className="font-semibold">{TRADE_UNKNOWN_PAIR_LINK_TITLE}</p>
      <p className="mt-1">
        {TRADE_UNKNOWN_PAIR_LINK_LEAD}{' '}
        <span className="font-mono text-xs opacity-90" data-testid="trade-pair-not-found-link-value">
          (“{display}”)
        </span>
      </p>
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <button
          type="button"
          className="btn-neo text-xs"
          data-testid="trade-pair-not-found-link-cta"
          onClick={focusPairSelect}
        >
          {TRADE_UNKNOWN_PAIR_LINK_CTA}
        </button>
        {onDismiss && (
          <button type="button" className="text-xs underline opacity-80" onClick={onDismiss}>
            Dismiss
          </button>
        )}
      </div>
    </div>
  )
}
