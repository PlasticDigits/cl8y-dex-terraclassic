type MarketDataServiceOutageBannerProps = {
  title: string
  lead: string
  tail?: string
  onRetry?: () => void
  testId: string
  /** Trade uses inline title + body; other routes use stacked heading + paragraph. */
  layout?: 'inline' | 'stacked'
}

export function MarketDataServiceOutageBanner({
  title,
  lead,
  tail,
  onRetry,
  testId,
  layout = 'stacked',
}: MarketDataServiceOutageBannerProps) {
  if (layout === 'inline') {
    return (
      <div className="alert-warning text-sm" role="alert" data-testid={testId}>
        <span className="font-semibold">{title}</span> {lead}
        {tail ? ` ${tail}` : null}
      </div>
    )
  }

  return (
    <div className="alert-warning" role="alert" data-testid={testId}>
      <p className="text-sm font-semibold uppercase tracking-wide font-heading" style={{ color: 'var(--ink)' }}>
        {title}
      </p>
      <p className="text-xs mt-2 leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
        {lead}
        {tail ? ` ${tail}` : null}
      </p>
      {onRetry && (
        <button type="button" className="btn-primary btn-cta !text-xs !px-4 !py-1.5 mt-3" onClick={onRetry}>
          Retry
        </button>
      )}
    </div>
  )
}
