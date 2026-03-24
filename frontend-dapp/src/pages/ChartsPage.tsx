import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { getOverview, getPairs, getPairStats, getTrades, getLeaderboard, INDEXER_URL } from '@/services/indexer/client'
import PriceChart from '@/components/charts/PriceChart'
import { StatBox, TradesTable, RetryError, Skeleton } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import { formatNum } from '@/utils/formatAmount'
import { shortenAddress } from '@/utils/tokenDisplay'
import { formatTime, formatTimeFromUnixSeconds } from '@/utils/formatDate'
import { getTwapPrices, getOracleInfo } from '@/services/terraclassic/oracle'
import type { IndexerPair, IndexerTrader } from '@/types'

const TWAP_WINDOWS = [
  { label: '5m', seconds: 300 },
  { label: '1h', seconds: 3600 },
  { label: '24h', seconds: 86400 },
]

const LEADERBOARD_TABS = [
  { key: 'total_volume', label: 'Volume' },
  { key: 'best_trade_pnl', label: 'Best Trade' },
  { key: 'total_realized_pnl', label: 'Most Profit' },
  { key: 'worst_trade_pnl', label: 'Most Loss' },
] as const

export default function ChartsPage() {
  const [selectedPairAddr, setSelectedPairAddr] = useState<string>('')
  const [leaderboardSort, setLeaderboardSort] = useState<string>('total_volume')

  const overviewQuery = useQuery({
    queryKey: ['indexer-overview'],
    queryFn: getOverview,
    refetchInterval: 30_000,
  })

  const pairsQuery = useQuery({
    queryKey: ['indexer-pairs'],
    queryFn: getPairs,
    staleTime: 60_000,
  })

  const pairs = pairsQuery.data ?? []
  const activePairAddr = selectedPairAddr || pairs[0]?.pair_address || ''
  const activePair = pairs.find((p: IndexerPair) => p.pair_address === activePairAddr)

  const statsQuery = useQuery({
    queryKey: ['pair-stats', activePairAddr],
    queryFn: () => getPairStats(activePairAddr),
    enabled: !!activePairAddr,
    refetchInterval: 30_000,
  })

  const tradesQuery = useQuery({
    queryKey: ['pair-trades', activePairAddr],
    queryFn: () => getTrades(activePairAddr, 50),
    enabled: !!activePairAddr,
    refetchInterval: 15_000,
  })

  const leaderboardQuery = useQuery({
    queryKey: ['leaderboard', leaderboardSort],
    queryFn: () => getLeaderboard(leaderboardSort, 20),
    refetchInterval: 30_000,
  })

  const twapQuery = useQuery({
    queryKey: ['twap-prices', activePairAddr],
    queryFn: () => getTwapPrices(activePairAddr, TWAP_WINDOWS),
    enabled: !!activePairAddr,
    staleTime: 30_000,
    refetchInterval: 60_000,
    retry: false,
  })

  const oracleInfoQuery = useQuery({
    queryKey: ['oracle-info', activePairAddr],
    queryFn: () => getOracleInfo(activePairAddr),
    enabled: !!activePairAddr,
    staleTime: 60_000,
    retry: false,
  })

  const overview = overviewQuery.data
  const stats = statsQuery.data

  const indexerUnavailable = pairsQuery.isError || overviewQuery.isError

  return (
    <div className="space-y-4">
      <h1 className="text-lg font-bold uppercase tracking-wider font-heading" style={{ color: 'var(--ink)' }}>
        Charts & Analytics
      </h1>

      {indexerUnavailable && (
        <div
          className="shell-panel border-2 border-amber-500/40"
          style={{ background: 'var(--panel-bg-strong)' }}
          role="alert"
        >
          <p className="text-sm font-semibold uppercase tracking-wide font-heading" style={{ color: 'var(--ink)' }}>
            Indexer unavailable
          </p>
          <p className="text-xs mt-2 leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
            Charts and analytics require the indexer HTTP API at{' '}
            <code className="font-mono text-[11px] px-1 py-0.5 border border-white/20">{INDEXER_URL}</code>. Start the
            indexer service, or set{' '}
            <code className="font-mono text-[11px] px-1 py-0.5 border border-white/20">VITE_INDEXER_URL</code> to match
            your deployment.
          </p>
          <button
            type="button"
            className="btn-muted !text-xs !px-4 !py-1.5 mt-3"
            onClick={() => {
              void overviewQuery.refetch()
              void pairsQuery.refetch()
            }}
          >
            Retry
          </button>
        </div>
      )}

      {/* Overview Bar */}
      <div className="shell-panel grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatBox
          label="24h Volume"
          value={overview ? formatNum(overview.total_volume_24h) : '—'}
          loading={overviewQuery.isLoading}
        />
        <StatBox
          label="24h Trades"
          value={overview ? overview.total_trades_24h.toLocaleString() : '—'}
          loading={overviewQuery.isLoading}
        />
        <StatBox
          label="Pairs"
          value={overview ? overview.pair_count.toString() : '—'}
          loading={overviewQuery.isLoading}
        />
        <StatBox
          label="Tokens"
          value={overview ? overview.token_count.toString() : '—'}
          loading={overviewQuery.isLoading}
        />
      </div>

      {/* Pair Selector */}
      <div className="shell-panel">
        <label className="label-neo mb-1 block">Select Pair</label>
        <select
          className="select-neo w-full"
          aria-label="Select pair"
          value={activePairAddr}
          onChange={(e) => {
            sounds.playButtonPress()
            setSelectedPairAddr(e.target.value)
          }}
        >
          {pairs.map((p: IndexerPair) => (
            <option key={p.pair_address} value={p.pair_address}>
              {p.asset_0.symbol} / {p.asset_1.symbol}
            </option>
          ))}
          {pairs.length === 0 && <option value="">No pairs available</option>}
        </select>
        {pairsQuery.isSuccess && pairs.length === 0 && !pairsQuery.isLoading && !indexerUnavailable && (
          <p className="text-xs mt-2 leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
            No pairs in the indexer yet. After swaps are indexed, pairs will appear here.
          </p>
        )}
      </div>

      {/* Price Chart */}
      {activePairAddr && <PriceChart pairAddress={activePairAddr} />}

      {/* 24h Stats */}
      {stats && activePair && (
        <div className="shell-panel">
          <h3
            className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading"
            style={{ color: 'var(--ink)' }}
          >
            24h Stats — {activePair.asset_0.symbol}/{activePair.asset_1.symbol}
          </h3>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
            <StatBox label="Volume (Base)" value={formatNum(stats.volume_base)} />
            <StatBox label="Volume (Quote)" value={formatNum(stats.volume_quote)} />
            <StatBox label="Trades" value={stats.trade_count.toLocaleString()} />
            <StatBox
              label="Price Change"
              value={
                stats.price_change_pct != null
                  ? `${stats.price_change_pct >= 0 ? '+' : ''}${stats.price_change_pct.toFixed(2)}%`
                  : '—'
              }
              color={
                stats.price_change_pct != null
                  ? stats.price_change_pct >= 0
                    ? 'var(--color-positive)'
                    : 'var(--color-negative)'
                  : undefined
              }
            />
            <StatBox label="High" value={stats.high ? formatNum(stats.high, 6) : '—'} />
            <StatBox label="Low" value={stats.low ? formatNum(stats.low, 6) : '—'} />
            <StatBox label="Open" value={stats.open_price ? formatNum(stats.open_price, 6) : '—'} />
            <StatBox label="Close" value={stats.close_price ? formatNum(stats.close_price, 6) : '—'} />
          </div>
        </div>
      )}

      {!statsQuery.isLoading && !stats && activePairAddr && (
        <div className="shell-panel text-center py-6" style={{ color: 'var(--ink-dim)' }}>
          <p className="text-sm uppercase tracking-wide font-medium">No Trading Data Yet</p>
          <p className="text-xs mt-1">Chart data will appear after the first trades are indexed for this pair.</p>
        </div>
      )}

      {/* TWAP Oracle Prices */}
      {activePairAddr && activePair && (
        <div className="shell-panel">
          <h3
            className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading"
            style={{ color: 'var(--ink)' }}
          >
            TWAP Oracle — {activePair.asset_0.symbol}/{activePair.asset_1.symbol}
          </h3>
          <div className="grid grid-cols-3 gap-3">
            {TWAP_WINDOWS.map((w) => {
              const entry = twapQuery.data?.find((e) => e.label === w.label)
              return (
                <StatBox
                  key={w.label}
                  label={`TWAP ${w.label}`}
                  value={entry?.price != null ? formatNum(entry.price, 6) : '—'}
                  loading={twapQuery.isLoading}
                />
              )
            })}
          </div>
          {oracleInfoQuery.data && (
            <div className="mt-3 grid grid-cols-2 md:grid-cols-4 gap-3">
              <StatBox
                label="Observations"
                value={`${oracleInfoQuery.data.observations_stored} / ${oracleInfoQuery.data.observation_cardinality}`}
              />
              <StatBox
                label="Oldest Obs."
                value={
                  oracleInfoQuery.data.oldest_observation_timestamp > 0
                    ? formatTimeFromUnixSeconds(oracleInfoQuery.data.oldest_observation_timestamp)
                    : '—'
                }
              />
              <StatBox
                label="Newest Obs."
                value={
                  oracleInfoQuery.data.newest_observation_timestamp > 0
                    ? formatTimeFromUnixSeconds(oracleInfoQuery.data.newest_observation_timestamp)
                    : '—'
                }
              />
              <StatBox label="Ring Buffer" value={oracleInfoQuery.data.observation_cardinality.toString()} />
            </div>
          )}
          {twapQuery.isError && (
            <p className="text-xs mt-2" style={{ color: 'var(--ink-subtle)' }}>
              Oracle data unavailable for this pair
            </p>
          )}
          {!twapQuery.isLoading && !twapQuery.isError && twapQuery.data?.every((e) => e.price === null) && (
            <p className="text-xs mt-2" style={{ color: 'var(--ink-subtle)' }}>
              Oracle observations are still accumulating. TWAP data will be available after sufficient trading activity.
            </p>
          )}
        </div>
      )}

      {/* Recent Trades */}
      <div className="shell-panel-strong">
        <h3 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
          Recent Trades
        </h3>
        {tradesQuery.isLoading && (
          <div className="space-y-2 py-4" aria-live="polite">
            {Array.from({ length: 5 }).map((_, i) => (
              <Skeleton key={i} height="1.5rem" />
            ))}
          </div>
        )}
        {tradesQuery.isError && (
          <RetryError message="Failed to load trades" onRetry={() => void tradesQuery.refetch()} />
        )}
        {tradesQuery.data && (
          <TradesTable
            trades={tradesQuery.data}
            formatTimeFn={formatTime}
            activePair={activePair}
            ariaLabel="Recent trades"
          />
        )}
      </div>

      {/* Leaderboard */}
      <div className="shell-panel-strong">
        <h3 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
          Leaderboard
        </h3>

        <div className="flex gap-1 mb-4 flex-wrap" role="tablist" aria-label="Leaderboard sort">
          {LEADERBOARD_TABS.map((tab) => (
            <button
              key={tab.key}
              role="tab"
              aria-selected={leaderboardSort === tab.key}
              onClick={() => {
                sounds.playButtonPress()
                setLeaderboardSort(tab.key)
              }}
              className={`tab-neo !text-[10px] !px-3 !py-1.5 ${
                leaderboardSort === tab.key ? 'tab-neo-active' : 'tab-neo-inactive'
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
        {leaderboardQuery.data && leaderboardQuery.data.length === 0 && (
          <p className="text-center py-8 text-sm" style={{ color: 'var(--ink-dim)' }}>
            No traders yet
          </p>
        )}
        {leaderboardQuery.data && leaderboardQuery.data.length > 0 && (
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
                    {LEADERBOARD_TABS.find((t) => t.key === leaderboardSort)?.label ?? 'Value'}
                  </th>
                  <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                    Trades
                  </th>
                </tr>
              </thead>
              <tbody>
                {leaderboardQuery.data.map((trader: IndexerTrader, i: number) => {
                  const metricValue = getLeaderboardMetric(trader, leaderboardSort)
                  const isPnl = leaderboardSort !== 'total_volume'
                  const numVal = parseFloat(metricValue) || 0
                  return (
                    <tr key={trader.address} className="border-b border-white/5 hover:bg-white/5 transition-colors">
                      <td className="py-1.5 px-2 font-semibold" style={{ color: 'var(--ink-subtle)' }}>
                        {i + 1}
                      </td>
                      <td className="py-1.5 px-2">
                        <Link
                          to={`/trader/${trader.address}`}
                          className="hover:underline"
                          style={{ color: 'var(--accent)' }}
                          onClick={() => sounds.playButtonPress()}
                        >
                          {shortenAddress(trader.address, 10, 6)}
                        </Link>
                      </td>
                      <td
                        className="py-1.5 px-2 text-right font-medium"
                        style={{
                          color: isPnl
                            ? numVal >= 0
                              ? 'var(--color-positive)'
                              : 'var(--color-negative)'
                            : 'var(--ink)',
                        }}
                      >
                        {formatNum(metricValue)}
                      </td>
                      <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>
                        {trader.total_trades.toLocaleString()}
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  )
}

function getLeaderboardMetric(trader: IndexerTrader, sort: string): string {
  switch (sort) {
    case 'best_trade_pnl':
      return trader.best_trade_pnl
    case 'total_realized_pnl':
      return trader.total_realized_pnl
    case 'worst_trade_pnl':
      return trader.worst_trade_pnl
    default:
      return trader.total_volume
  }
}
