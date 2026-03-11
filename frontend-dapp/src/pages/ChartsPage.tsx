import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import {
  getOverview,
  getPairs,
  getPairStats,
  getTrades,
  getLeaderboard,
} from '@/services/indexer/client'
import PriceChart from '@/components/charts/PriceChart'
import { Spinner } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import type { IndexerPair, IndexerTrade, IndexerTrader } from '@/types'

const LEADERBOARD_TABS = [
  { key: 'total_volume', label: 'Volume' },
  { key: 'best_trade_pnl', label: 'Best Trade' },
  { key: 'total_realized_pnl', label: 'Most Profit' },
  { key: 'worst_trade_pnl', label: 'Most Loss' },
] as const

function formatNum(val: string | number, decimals = 2): string {
  const n = typeof val === 'string' ? parseFloat(val) : val
  if (isNaN(n) || n === 0) return '0'
  if (Math.abs(n) >= 1e9) return (n / 1e9).toFixed(decimals) + 'B'
  if (Math.abs(n) >= 1e6) return (n / 1e6).toFixed(decimals) + 'M'
  if (Math.abs(n) >= 1e3) return (n / 1e3).toFixed(decimals) + 'K'
  return n.toFixed(decimals)
}

function truncAddr(addr: string): string {
  if (addr.length <= 16) return addr
  return addr.slice(0, 10) + '...' + addr.slice(-6)
}

function formatTime(iso: string): string {
  const d = new Date(iso)
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

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
  const activePairAddr = selectedPairAddr || pairs[0]?.contract_address || ''
  const activePair = pairs.find((p: IndexerPair) => p.contract_address === activePairAddr)

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

  const overview = overviewQuery.data
  const stats = statsQuery.data

  return (
    <div className="space-y-4">
      <h1
        className="text-lg font-bold uppercase tracking-wider"
        style={{ color: 'var(--ink)', fontFamily: "'Chakra Petch', sans-serif" }}
      >
        Charts & Analytics
      </h1>

      {/* Overview Bar */}
      <div className="shell-panel grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatBox label="24h Volume" value={overview ? formatNum(overview.total_volume_24h) : '—'} loading={overviewQuery.isLoading} />
        <StatBox label="24h Trades" value={overview ? overview.total_trades_24h.toLocaleString() : '—'} loading={overviewQuery.isLoading} />
        <StatBox label="Pairs" value={overview ? overview.pair_count.toString() : '—'} loading={overviewQuery.isLoading} />
        <StatBox label="Tokens" value={overview ? overview.token_count.toString() : '—'} loading={overviewQuery.isLoading} />
      </div>

      {/* Pair Selector */}
      <div className="shell-panel">
        <label className="label-neo mb-1 block">Select Pair</label>
        <select
          className="select-neo w-full"
          value={activePairAddr}
          onChange={(e) => {
            sounds.playButtonPress()
            setSelectedPairAddr(e.target.value)
          }}
        >
          {pairs.map((p: IndexerPair) => (
            <option key={p.contract_address} value={p.contract_address}>
              {p.asset_0.symbol} / {p.asset_1.symbol}
            </option>
          ))}
          {pairs.length === 0 && <option value="">No pairs available</option>}
        </select>
      </div>

      {/* Price Chart */}
      {activePairAddr && <PriceChart pairAddress={activePairAddr} />}

      {/* 24h Stats */}
      {stats && activePair && (
        <div className="shell-panel">
          <h3
            className="text-sm font-semibold uppercase tracking-wide mb-3"
            style={{ color: 'var(--ink)', fontFamily: "'Chakra Petch', sans-serif" }}
          >
            24h Stats — {activePair.asset_0.symbol}/{activePair.asset_1.symbol}
          </h3>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
            <StatBox label="Volume (Base)" value={formatNum(stats.volume_base)} />
            <StatBox label="Volume (Quote)" value={formatNum(stats.volume_quote)} />
            <StatBox label="Trades" value={stats.trade_count.toLocaleString()} />
            <StatBox
              label="Price Change"
              value={stats.price_change_pct != null ? `${stats.price_change_pct >= 0 ? '+' : ''}${stats.price_change_pct.toFixed(2)}%` : '—'}
              color={stats.price_change_pct != null ? (stats.price_change_pct >= 0 ? '#22c55e' : '#ef4444') : undefined}
            />
            <StatBox label="High" value={stats.high ? formatNum(stats.high, 6) : '—'} />
            <StatBox label="Low" value={stats.low ? formatNum(stats.low, 6) : '—'} />
            <StatBox label="Open" value={stats.open_price ? formatNum(stats.open_price, 6) : '—'} />
            <StatBox label="Close" value={stats.close_price ? formatNum(stats.close_price, 6) : '—'} />
          </div>
        </div>
      )}

      {/* Recent Trades */}
      <div className="shell-panel-strong">
        <h3
          className="text-sm font-semibold uppercase tracking-wide mb-3"
          style={{ color: 'var(--ink)', fontFamily: "'Chakra Petch', sans-serif" }}
        >
          Recent Trades
        </h3>
        {tradesQuery.isLoading && (
          <div className="flex items-center justify-center py-8 gap-2" style={{ color: 'var(--ink-subtle)' }}>
            <Spinner /> Loading trades...
          </div>
        )}
        {tradesQuery.data && tradesQuery.data.length === 0 && (
          <p className="text-center py-8 text-sm" style={{ color: 'var(--ink-dim)' }}>No trades yet</p>
        )}
        {tradesQuery.data && tradesQuery.data.length > 0 && (
          <div className="overflow-x-auto">
            <table className="w-full text-xs">
              <thead>
                <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
                  <th className="text-left py-2 px-2 font-medium uppercase tracking-wider">Time</th>
                  <th className="text-left py-2 px-2 font-medium uppercase tracking-wider">Direction</th>
                  <th className="text-right py-2 px-2 font-medium uppercase tracking-wider">Offer</th>
                  <th className="text-right py-2 px-2 font-medium uppercase tracking-wider">Return</th>
                  <th className="text-right py-2 px-2 font-medium uppercase tracking-wider">Price</th>
                  <th className="text-left py-2 px-2 font-medium uppercase tracking-wider">Tx</th>
                </tr>
              </thead>
              <tbody>
                {tradesQuery.data.map((t: IndexerTrade) => {
                  const isBuy = activePair && t.offer_asset === activePair.asset_0.symbol
                  return (
                    <tr key={t.id} className="border-b border-white/5 hover:bg-white/5 transition-colors">
                      <td className="py-1.5 px-2" style={{ color: 'var(--ink-subtle)' }}>{formatTime(t.block_timestamp)}</td>
                      <td className="py-1.5 px-2 font-medium" style={{ color: isBuy ? '#22c55e' : '#ef4444' }}>
                        {t.offer_asset} → {t.ask_asset}
                      </td>
                      <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink)' }}>{formatNum(t.offer_amount)}</td>
                      <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink)' }}>{formatNum(t.return_amount)}</td>
                      <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>{formatNum(t.price, 6)}</td>
                      <td className="py-1.5 px-2" style={{ color: 'var(--ink-dim)' }}>{t.tx_hash.slice(0, 8)}...</td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* Leaderboard */}
      <div className="shell-panel-strong">
        <h3
          className="text-sm font-semibold uppercase tracking-wide mb-3"
          style={{ color: 'var(--ink)', fontFamily: "'Chakra Petch', sans-serif" }}
        >
          Leaderboard
        </h3>

        <div className="flex gap-1 mb-4 flex-wrap">
          {LEADERBOARD_TABS.map((tab) => (
            <button
              key={tab.key}
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
          <div className="flex items-center justify-center py-8 gap-2" style={{ color: 'var(--ink-subtle)' }}>
            <Spinner /> Loading leaderboard...
          </div>
        )}
        {leaderboardQuery.data && leaderboardQuery.data.length === 0 && (
          <p className="text-center py-8 text-sm" style={{ color: 'var(--ink-dim)' }}>No traders yet</p>
        )}
        {leaderboardQuery.data && leaderboardQuery.data.length > 0 && (
          <div className="overflow-x-auto">
            <table className="w-full text-xs">
              <thead>
                <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
                  <th className="text-left py-2 px-2 font-medium uppercase tracking-wider">#</th>
                  <th className="text-left py-2 px-2 font-medium uppercase tracking-wider">Trader</th>
                  <th className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                    {LEADERBOARD_TABS.find((t) => t.key === leaderboardSort)?.label ?? 'Value'}
                  </th>
                  <th className="text-right py-2 px-2 font-medium uppercase tracking-wider">Trades</th>
                </tr>
              </thead>
              <tbody>
                {leaderboardQuery.data.map((trader: IndexerTrader, i: number) => {
                  const metricValue = getLeaderboardMetric(trader, leaderboardSort)
                  const isPnl = leaderboardSort !== 'total_volume'
                  const numVal = parseFloat(metricValue)
                  return (
                    <tr key={trader.address} className="border-b border-white/5 hover:bg-white/5 transition-colors">
                      <td className="py-1.5 px-2 font-semibold" style={{ color: 'var(--ink-subtle)' }}>{i + 1}</td>
                      <td className="py-1.5 px-2">
                        <Link
                          to={`/trader/${trader.address}`}
                          className="hover:underline"
                          style={{ color: 'var(--accent)' }}
                          onClick={() => sounds.playButtonPress()}
                        >
                          {truncAddr(trader.address)}
                        </Link>
                      </td>
                      <td
                        className="py-1.5 px-2 text-right font-medium"
                        style={{ color: isPnl ? (numVal >= 0 ? '#22c55e' : '#ef4444') : 'var(--ink)' }}
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

function StatBox({ label, value, loading, color }: { label: string; value: string; loading?: boolean; color?: string }) {
  return (
    <div className="p-3 border border-white/10 rounded-sm" style={{ background: 'var(--panel-bg)' }}>
      <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>{label}</p>
      {loading ? (
        <Spinner />
      ) : (
        <p className="text-sm font-bold" style={{ color: color ?? 'var(--ink)', fontFamily: "'Chakra Petch', sans-serif" }}>
          {value}
        </p>
      )}
    </div>
  )
}

function getLeaderboardMetric(trader: IndexerTrader, sort: string): string {
  switch (sort) {
    case 'best_trade_pnl': return trader.best_trade_pnl
    case 'total_realized_pnl': return trader.total_realized_pnl
    case 'worst_trade_pnl': return trader.worst_trade_pnl
    default: return trader.total_volume
  }
}
