import { useState } from 'react'
import { RetryError } from '@/components/ui/RetryError'
import { Skeleton } from '@/components/ui/Skeleton'
import { PnlValue } from '@/components/trader/PnlValue'
import { TraderIdentity } from '@/components/trader/TraderIdentity'
import { sounds } from '@/lib/sounds'
import { formatIndexedVolumeUsd } from '@/utils/chartsOverviewStats'
import {
  DEFAULT_LEADERBOARD_SORT,
  LEADERBOARD_TABS,
  PAIR_LEADERBOARD_TABS,
  filterLeaderboardRows,
  getLeaderboardPnlValue,
  isLeaderboardSort,
  type LeaderboardSort,
} from './traderLeaderboard'
import { useTraderLeaderboardQuery } from './useTraderLeaderboardQuery'

export type TraderLeaderboardProps = {
  /** When this wallet appears in the top 20, mark that row current. No invented rank (TL-9). */
  highlightAddress?: string
  /** Indexed pair contract. Charts #666. Omit for DEX-wide #657. */
  pairAddress?: string
  enabled?: boolean
}

/**
 * DEX trader leaderboard. Unscoped `GET /traders/leaderboard` on `/trader`.
 * Charts passes `pairAddress` for pair ranks (#666 CS-7 / CS-14).
 */
export function TraderLeaderboard({ highlightAddress, pairAddress, enabled = true }: TraderLeaderboardProps) {
  const pairTrim = pairAddress?.trim() || ''
  const tabs = pairTrim ? PAIR_LEADERBOARD_TABS : LEADERBOARD_TABS
  const [leaderboardSort, setLeaderboardSort] = useState<LeaderboardSort>(DEFAULT_LEADERBOARD_SORT)
  const sortAllowed = tabs.some((tab) => tab.key === leaderboardSort)
  const sort: LeaderboardSort =
    sortAllowed && isLeaderboardSort(leaderboardSort) ? leaderboardSort : DEFAULT_LEADERBOARD_SORT

  const leaderboardQuery = useTraderLeaderboardQuery(sort, pairTrim || undefined, enabled)

  const rows = filterLeaderboardRows(leaderboardQuery.data)
  const highlight = highlightAddress?.trim().toLowerCase() ?? ''

  return (
    <div className="shell-panel-strong" data-testid="trader-leaderboard">
      <h3 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
        Leaderboard
      </h3>

      <div className="flex gap-1 mb-4 flex-wrap" role="tablist" aria-label="Leaderboard sort">
        {tabs.map((tab) => (
          <button
            key={tab.key}
            type="button"
            role="tab"
            aria-selected={sort === tab.key}
            onClick={() => {
              sounds.playButtonPress()
              setLeaderboardSort(tab.key)
            }}
            className={`tab-glass !text-[10px] !px-3 !py-1.5 ${
              sort === tab.key ? 'tab-glass-active' : 'tab-glass-inactive'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {leaderboardQuery.isLoading && (
        <div className="space-y-2 py-4" aria-live="polite">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} height="1.5rem" />
          ))}
        </div>
      )}
      {leaderboardQuery.isError && (
        <RetryError message="Failed to load leaderboard" onRetry={() => void leaderboardQuery.refetch()} />
      )}
      {leaderboardQuery.data && rows.length === 0 && !leaderboardQuery.isError && (
        <p className="text-center py-8 text-sm" style={{ color: 'var(--ink-dim)' }}>
          {pairTrim ? 'No traders on this pair yet' : 'No traders yet'}
        </p>
      )}
      {rows.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-xs" aria-label="Trader leaderboard">
            <thead>
              <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
                <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">
                  #
                </th>
                <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">
                  Trader
                </th>
                <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                  {tabs.find((t) => t.key === sort)?.label ?? 'Value'}
                </th>
                <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                  Trades
                </th>
              </tr>
            </thead>
            <tbody>
              {rows.map((trader, i) => {
                const isPnl = sort !== 'total_volume_usd'
                const isCurrent = highlight !== '' && trader.address.toLowerCase() === highlight
                const trades =
                  typeof trader.total_trades === 'number' && Number.isFinite(trader.total_trades)
                    ? trader.total_trades
                    : 0
                return (
                  <tr
                    key={trader.address}
                    className={`border-b border-white/5 transition-colors ${
                      isCurrent ? 'bg-white/10' : 'hover:bg-white/5'
                    }`}
                    aria-current={isCurrent ? 'page' : undefined}
                  >
                    <td className="py-1.5 px-2 font-semibold" style={{ color: 'var(--ink-subtle)' }}>
                      {i + 1}
                    </td>
                    <td className="py-1.5 px-2">
                      <TraderIdentity
                        address={trader.address}
                        linkToProfile
                        onClick={() => sounds.playButtonPress()}
                        data-testid="charts-leaderboard-trader"
                      />
                    </td>
                    <td
                      className="py-1.5 px-2 text-right font-medium"
                      style={{ color: isPnl ? undefined : 'var(--ink)' }}
                    >
                      {isPnl ? (
                        <PnlValue value={getLeaderboardPnlValue(trader, sort)} />
                      ) : (
                        <span data-testid="charts-leaderboard-volume">
                          {formatIndexedVolumeUsd(trader.total_volume_usd, trades)}
                        </span>
                      )}
                    </td>
                    <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>
                      {trades.toLocaleString()}
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}

export default TraderLeaderboard
