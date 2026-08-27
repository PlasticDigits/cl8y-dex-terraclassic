import type { ReactNode } from 'react'
import { Fragment } from 'react'
import { Link } from 'react-router-dom'
import { RetryError } from '@/components/ui/RetryError'
import { Skeleton } from '@/components/ui/Skeleton'
import { useTraderUsdMarks } from '@/hooks/useTraderUsdMarks'
import { sounds } from '@/lib/sounds'
import type { IndexerPosition } from '@/types'
import { isTestPosition } from '@/utils/portfolioPerformanceFilter'
import { formatScaledPosition, NO_COST_BASIS_LABEL, type TraderUsdMarks } from '@/utils/traderPositionDisplay'

const POSITION_COL_COUNT = 8

export type TraderPositionsTableProps = {
  positions: IndexerPosition[] | undefined
  isLoading: boolean
  isError: boolean
  onRetry: () => void
  emptyMessage?: string
  sectionTestId?: string
  /** Test override. Live table uses {@link useTraderUsdMarks}. */
  usdMarks?: TraderUsdMarks
  /** Optional hatch (portfolio #674). `/trader` leaves this unset. */
  headerAction?: ReactNode
  /** Insert a #534-style Test pairs divider before the first gem row. */
  showTestPairDivider?: boolean
}

/** Open quote positions from `GET /api/v1/traders/{addr}/positions` (GitLab #212 / #551 / #674 / #675). */
export function TraderPositionsTable({
  positions,
  isLoading,
  isError,
  onRetry,
  emptyMessage = 'No open positions.',
  sectionTestId = 'trader-positions-section',
  usdMarks: usdMarksProp,
  headerAction,
  showTestPairDivider = false,
}: TraderPositionsTableProps) {
  const liveMarks = useTraderUsdMarks()
  const usdMarks = usdMarksProp ?? liveMarks
  return (
    <div className="shell-panel-strong" data-testid={sectionTestId}>
      <div className="flex flex-wrap items-start justify-between gap-2 mb-1">
        <h3 className="text-sm font-semibold uppercase tracking-wide font-heading" style={{ color: 'var(--ink)' }}>
          Open positions
        </h3>
        {headerAction}
      </div>
      <p className="text-xs mb-3" style={{ color: 'var(--ink-dim)' }}>
        Net quote exposure per pair, in that pair&apos;s tokens. Mark is the current hub value of remaining quote.
        Unrealized is that mark minus on-DEX cost — not wallet balances. LP is on{' '}
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
                  Mark
                </th>
                <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                  Unrealized P&L
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
              {positions.map((pos, index) => {
                const scaled = formatScaledPosition(pos, usdMarks)
                const unrealizedTone = scaled.unrealizedPnl.startsWith('+')
                  ? 'var(--color-positive)'
                  : scaled.unrealizedPnl.startsWith('-')
                    ? 'var(--color-negative)'
                    : 'var(--ink-subtle)'
                const showDivider =
                  showTestPairDivider && isTestPosition(pos) && (index === 0 || !isTestPosition(positions[index - 1]!))
                return (
                  <Fragment key={pos.pair_address}>
                    {showDivider ? (
                      <tr>
                        <td
                          colSpan={POSITION_COL_COUNT}
                          className="py-1.5 px-2 text-[10px] uppercase tracking-wide font-semibold"
                          style={{ color: 'var(--ink-dim)' }}
                          data-testid="trader-positions-test-pairs-divider"
                        >
                          Test pairs
                        </td>
                      </tr>
                    ) : null}
                    <tr className="border-b border-white/5 hover:bg-white/5 transition-colors">
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
                      <td
                        className="py-1.5 px-2 text-right"
                        style={{ color: 'var(--ink)' }}
                        data-testid="trader-position-net"
                        title="Human quote token amount"
                      >
                        {scaled.netPosition}
                      </td>
                      <td
                        className="py-1.5 px-2 text-right"
                        style={{ color: 'var(--ink-subtle)' }}
                        data-testid="trader-position-avg-entry"
                        title="Human base paid per 1 human quote. Not USD."
                      >
                        {scaled.avgEntry}
                      </td>
                      <td
                        className="py-1.5 px-2 text-right"
                        style={{ color: 'var(--ink-subtle)' }}
                        data-testid="trader-position-cost"
                        title="Human base token spent"
                      >
                        {scaled.costBasis}
                      </td>
                      <td
                        className="py-1.5 px-2 text-right"
                        style={{ color: 'var(--ink)' }}
                        data-testid="trader-position-mark"
                        title="Current hub USD of remaining quote (DEX pool, not CEX)"
                      >
                        {scaled.markLabel}
                      </td>
                      <td
                        className="py-1.5 px-2 text-right"
                        data-testid="trader-position-unrealized"
                        title={
                          scaled.hasCostBasis
                            ? 'Hub mark minus on-DEX cost, in the pair base token'
                            : 'DEX never recorded a buy for this remaining quote'
                        }
                      >
                        <span className="font-bold font-heading" style={{ color: unrealizedTone }}>
                          {scaled.hasCostBasis ? scaled.unrealizedPnl : NO_COST_BASIS_LABEL}
                        </span>
                      </td>
                      <td
                        className="py-1.5 px-2 text-right"
                        data-testid="trader-position-pnl"
                        title="Human base token realized P&L"
                      >
                        <span
                          className="font-bold font-heading"
                          style={{
                            color: scaled.realizedPnl.startsWith('+')
                              ? 'var(--color-positive)'
                              : scaled.realizedPnl.startsWith('-')
                                ? 'var(--color-negative)'
                                : 'var(--ink-subtle)',
                          }}
                        >
                          {scaled.realizedPnl}
                        </span>
                      </td>
                      <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>
                        {pos.trade_count}
                      </td>
                    </tr>
                  </Fragment>
                )
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
