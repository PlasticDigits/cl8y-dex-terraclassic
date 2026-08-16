import { Link } from 'react-router-dom'
import { RetryError } from '@/components/ui/RetryError'
import { Skeleton } from '@/components/ui/Skeleton'
import { formatTokenAmount } from '@/utils/formatAmount'
import { sounds } from '@/lib/sounds'
import type { PortfolioLpRow } from '@/hooks/usePortfolioLpBalances'
import { POOL_LP_HOWTO_HREF, POOL_LP_HOWTO_SUMMARY } from '@/utils/poolLpHowtoCopy'

const LP_DECIMALS = 6

export type PortfolioLpOverviewSectionProps = {
  rows: PortfolioLpRow[] | undefined
  pairsScanned?: number
  capped?: boolean
  isLoading: boolean
  isError: boolean
  onRetry: () => void
}

export function PortfolioLpOverviewSection({
  rows,
  pairsScanned,
  capped,
  isLoading,
  isError,
  onRetry,
}: PortfolioLpOverviewSectionProps) {
  return (
    <div className="shell-panel-strong" data-testid="portfolio-lp-overview-section">
      <h3 className="text-sm font-semibold uppercase tracking-wide mb-1 font-heading" style={{ color: 'var(--ink)' }}>
        LP overview
      </h3>
      <p className="text-xs mb-3" style={{ color: 'var(--ink-dim)' }}>
        LP token balances. Manage on{' '}
        <Link to="/pool" className="underline" style={{ color: 'var(--accent)' }}>
          Pool
        </Link>
        {' · '}
        <Link
          to={POOL_LP_HOWTO_HREF}
          className="underline"
          style={{ color: 'var(--accent)' }}
          data-testid="portfolio-lp-howto-link"
        >
          {POOL_LP_HOWTO_SUMMARY}
        </Link>
        .
      </p>
      {capped && (
        <p className="text-xs mb-2" style={{ color: 'var(--ink-dim)' }} data-testid="portfolio-lp-capped-notice">
          Showing first {pairsScanned ?? '—'} pairs only.
        </p>
      )}
      {isLoading && (
        <div className="space-y-2 py-4" aria-live="polite">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} height="1.5rem" />
          ))}
        </div>
      )}
      {isError && <RetryError message="Failed to load LP balances" onRetry={onRetry} />}
      {!isLoading && !isError && rows && rows.length === 0 && (
        <p className="text-center py-6 text-sm" style={{ color: 'var(--ink-dim)' }} data-testid="portfolio-lp-empty">
          No LP balances found.
        </p>
      )}
      {!isLoading && !isError && rows && rows.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-xs" aria-label="LP token balances">
            <thead>
              <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
                <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">
                  Pair
                </th>
                <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                  LP balance
                </th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.pairAddress} className="border-b border-white/5 hover:bg-white/5 transition-colors">
                  <td className="py-1.5 px-2 font-medium" style={{ color: 'var(--ink)' }}>
                    <Link
                      to={`/pool`}
                      onClick={() => sounds.playButtonPress()}
                      className="no-underline hover:underline"
                      style={{ color: 'var(--accent)' }}
                    >
                      {row.label}
                    </Link>
                  </td>
                  <td className="py-1.5 px-2 text-right font-mono" style={{ color: 'var(--ink)' }}>
                    {formatTokenAmount(row.balanceRaw, LP_DECIMALS)}
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
