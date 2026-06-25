import { shortenAddress } from '@/utils/tokenDisplay'
import {
  TRADE_INVALID_PAIR_LINK_CTA,
  TRADE_INVALID_PAIR_LINK_LEAD,
  TRADE_INVALID_PAIR_LINK_TITLE,
} from '@/utils/tradeInvalidPairLinkCopy'

export interface InvalidPairLinkNoticeProps {
  /** Decoded invalid segment from the URL (shown quoted, truncated when long). */
  invalidParam: string
  pairSelectId: string
  onDismiss?: () => void
  className?: string
}

function formatInvalidParamDisplay(raw: string): string {
  const trimmed = raw.trim()
  if (trimmed.length <= 48) return trimmed
  return shortenAddress(trimmed, 10, 8)
}

export function InvalidPairLinkNotice({
  invalidParam,
  pairSelectId,
  onDismiss,
  className = '',
}: InvalidPairLinkNoticeProps) {
  const display = formatInvalidParamDisplay(invalidParam)

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
      data-testid="trade-invalid-pair-link-notice"
    >
      <p className="font-semibold">{TRADE_INVALID_PAIR_LINK_TITLE}</p>
      <p className="mt-1">
        {TRADE_INVALID_PAIR_LINK_LEAD}{' '}
        <span className="font-mono text-xs opacity-90" data-testid="trade-invalid-pair-link-value">
          (“{display}”)
        </span>
      </p>
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <button
          type="button"
          className="btn-muted text-xs"
          data-testid="trade-invalid-pair-link-cta"
          onClick={focusPairSelect}
        >
          {TRADE_INVALID_PAIR_LINK_CTA}
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
