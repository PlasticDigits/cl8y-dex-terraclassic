import { Spinner } from '@/components/ui'

/** Immediate feedback while indexer chart/book queries load after a pair change (GitLab #180). */
export function TradePairSwitchStatus({ pairLabel }: { pairLabel?: string }) {
  return (
    <div
      role="status"
      aria-live="polite"
      data-testid="trade-pair-switch-loading"
      className="flex items-center gap-2 rounded-lg border px-3 py-2 text-xs font-medium uppercase tracking-wide"
      style={{ borderColor: 'var(--line)', color: 'var(--ink-dim)' }}
    >
      <Spinner />
      <span>
        Loading market data
        {pairLabel ? ` for ${pairLabel}` : ''}…
      </span>
    </div>
  )
}
