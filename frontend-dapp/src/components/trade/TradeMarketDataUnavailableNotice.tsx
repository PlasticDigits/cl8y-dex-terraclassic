type TradeMarketDataUnavailableNoticeProps = {
  message: string
  'data-testid'?: string
}

/** Dashed empty row for trade panels when market data HTTP is unreachable (GitLab #165). */
export function TradeMarketDataUnavailableNotice({
  message,
  'data-testid': testId,
}: TradeMarketDataUnavailableNoticeProps) {
  return (
    <div
      className="flex min-h-[96px] items-center justify-center rounded-xl border border-dashed px-3 py-4 text-center text-[11px] leading-relaxed"
      style={{ borderColor: 'var(--line)', color: 'var(--ink-subtle)' }}
      data-testid={testId}
      role="status"
    >
      {message}
    </div>
  )
}
