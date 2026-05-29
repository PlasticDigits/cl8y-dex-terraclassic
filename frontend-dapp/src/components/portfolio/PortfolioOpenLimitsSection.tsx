import { Link } from 'react-router-dom'
import { RetryError } from '@/components/ui/RetryError'
import { Skeleton } from '@/components/ui/Skeleton'
import { formatNum } from '@/utils/formatAmount'
import { sounds } from '@/lib/sounds'
import type { IndexerLimitPlacement } from '@/types'
import { placementLifecycleLabel } from '@/utils/limitPlacementLifecycle'

export type PortfolioOpenLimitsSectionProps = {
  placements: IndexerLimitPlacement[] | undefined
  isLoading: boolean
  isError: boolean
  onRetry: () => void
}

export function PortfolioOpenLimitsSection({
  placements,
  isLoading,
  isError,
  onRetry,
}: PortfolioOpenLimitsSectionProps) {
  return (
    <div className="shell-panel-strong" data-testid="portfolio-open-limits-section">
      <h3 className="text-sm font-semibold uppercase tracking-wide mb-1 font-heading" style={{ color: 'var(--ink)' }}>
        Open limits
      </h3>
      <p className="text-xs mb-3" style={{ color: 'var(--ink-dim)' }}>
        Resting limit orders across all indexed pairs for your wallet. Cancel or claim refunds on{' '}
        <Link to="/limits" className="underline" style={{ color: 'var(--accent)' }}>
          Limits
        </Link>{' '}
        or per-pair on{' '}
        <Link to="/trade" className="underline" style={{ color: 'var(--accent)' }}>
          Trade
        </Link>
        .
      </p>
      {isLoading && (
        <div className="space-y-2 py-4" aria-live="polite">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} height="1.5rem" />
          ))}
        </div>
      )}
      {isError && <RetryError message="Failed to load open limits" onRetry={onRetry} />}
      {!isLoading && !isError && placements && placements.length === 0 && (
        <p
          className="text-center py-6 text-sm"
          style={{ color: 'var(--ink-dim)' }}
          data-testid="portfolio-open-limits-empty"
        >
          No open indexed limit placements for this wallet.
        </p>
      )}
      {!isLoading && !isError && placements && placements.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-xs" aria-label="Open limit orders">
            <thead>
              <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
                <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">
                  Pair
                </th>
                <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                  Order
                </th>
                <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">
                  Side
                </th>
                <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                  Price
                </th>
                <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">
                  Status
                </th>
                <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                  Placed
                </th>
              </tr>
            </thead>
            <tbody>
              {placements.map((row) => (
                <tr key={row.id} className="border-b border-white/5 hover:bg-white/5 transition-colors">
                  <td className="py-1.5 px-2 font-medium" style={{ color: 'var(--ink)' }}>
                    <Link
                      to={`/trade/${row.pair_address}`}
                      onClick={() => sounds.playButtonPress()}
                      className="no-underline hover:underline"
                      style={{ color: 'var(--accent)' }}
                    >
                      {row.pair_address.slice(0, 10)}…
                    </Link>
                  </td>
                  <td className="py-1.5 px-2 text-right font-mono" style={{ color: 'var(--ink-subtle)' }}>
                    #{row.order_id}
                  </td>
                  <td className="py-1.5 px-2 capitalize" style={{ color: 'var(--ink-subtle)' }}>
                    {row.side ?? '—'}
                  </td>
                  <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink)' }}>
                    {row.price != null ? formatNum(row.price, 6) : '—'}
                  </td>
                  <td className="py-1.5 px-2" style={{ color: 'var(--ink-subtle)' }}>
                    {placementLifecycleLabel(row)}
                  </td>
                  <td className="py-1.5 px-2 text-right whitespace-nowrap" style={{ color: 'var(--ink-subtle)' }}>
                    {row.block_timestamp.slice(0, 19)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
