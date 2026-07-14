import { Link } from 'react-router-dom'
import { RetryError } from '@/components/ui/RetryError'
import { Skeleton } from '@/components/ui/Skeleton'
import { formatNum } from '@/utils/formatAmount'
import { sounds } from '@/lib/sounds'
import type { IndexerPosition } from '@/types'
import { PnlValue } from './PnlValue'

export type TraderPositionsTableProps = {
  positions: IndexerPosition[] | undefined
  isLoading: boolean
  isError: boolean
  onRetry: () => void
  emptyMessage?: string
  sectionTestId?: string
}

/** Open quote positions from `GET /api/v1/traders/{addr}/positions` (GitLab #212). */
export function TraderPositionsTable({
  positions,
  isLoading,
  isError,
  onRetry,
  emptyMessage = 'No open positions.',
  sectionTestId = 'trader-positions-section',
}: TraderPositionsTableProps) {
  return (
    <div className="shell-panel-strong" data-testid={sectionTestId}>
      <h3 className="text-sm font-semibold uppercase tracking-wide mb-1 font-heading" style={{ color: 'var(--ink)' }}>
        Open positions
      </h3>
      <p className="text-xs mb-3" style={{ color: 'var(--ink-dim)' }}>
        Net exposure per pair. LP balances are on{' '}
        <Link to="/pool" className="underline" style={{ color: 'var(--accent)' }}>
          Pool
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
      {isError && <RetryError message="Failed to load positions" onRetry={onRetry} />}
      {!isLoading && !isError && positions && positions.length === 0 && (
        <p
          className="text-center py-6 text-sm"
          style={{ color: 'var(--ink-dim)' }}
          data-testid="trader-positions-empty"
        >
          {emptyMessage}
        </p>
      )}
      {!isLoading && !isError && positions && positions.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-xs" aria-label="Open positions">
            <thead>
              <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
                <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">
                  Pair
                </th>
                <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                  Net Position
                </th>
                <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                  Avg Entry
                </th>
                <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                  Cost Basis
                </th>
                <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                  Realized P&L
                </th>
                <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                  Trades
                </th>
              </tr>
            </thead>
            <tbody>
              {positions.map((pos) => (
                <tr key={pos.pair_address} className="border-b border-white/5 hover:bg-white/5 transition-colors">
                  <td className="py-1.5 px-2 font-medium" style={{ color: 'var(--ink)' }}>
                    <Link
                      to={`/trade/${pos.pair_address}`}
                      onClick={() => sounds.playButtonPress()}
                      className="no-underline hover:underline"
                      style={{ color: 'var(--accent)' }}
                    >
                      {pos.asset_0_symbol}/{pos.asset_1_symbol}
                    </Link>
                  </td>
                  <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink)' }}>
                    {formatNum(pos.net_position_quote, 4)}
                  </td>
                  <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>
                    {formatNum(pos.avg_entry_price, 6)}
                  </td>
                  <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>
                    {formatNum(pos.total_cost_base)}
                  </td>
                  <td className="py-1.5 px-2 text-right">
                    <PnlValue value={pos.realized_pnl} />
                  </td>
                  <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>
                    {pos.trade_count}
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
