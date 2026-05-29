import { Spinner } from '@/components/ui'

type MarketDataLoadingStatusProps = {
  label: string
  testId?: string
}

/** Shared polite loading affordance for indexer-backed surfaces (GitLab #180, #215). */
export function MarketDataLoadingStatus({ label, testId = 'market-data-loading' }: MarketDataLoadingStatusProps) {
  return (
    <div
      role="status"
      aria-live="polite"
      data-testid={testId}
      className="flex items-center gap-2 rounded-lg border px-3 py-2 text-xs font-medium uppercase tracking-wide"
      style={{ borderColor: 'var(--line)', color: 'var(--ink-dim)' }}
    >
      <Spinner />
      <span>{label}</span>
    </div>
  )
}
