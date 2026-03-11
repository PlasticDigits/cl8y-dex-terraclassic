import { useState } from 'react'
import { useParams, useNavigate, Link } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import {
  getTrader,
  getTraderTrades,
  getTraderPositions,
} from '@/services/indexer/client'
import { Spinner } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import type { IndexerTrade, IndexerPosition } from '@/types'

function formatNum(val: string | number, decimals = 2): string {
  const n = typeof val === 'string' ? parseFloat(val) : val
  if (isNaN(n) || n === 0) return '0'
  if (Math.abs(n) >= 1e9) return (n / 1e9).toFixed(decimals) + 'B'
  if (Math.abs(n) >= 1e6) return (n / 1e6).toFixed(decimals) + 'M'
  if (Math.abs(n) >= 1e3) return (n / 1e3).toFixed(decimals) + 'K'
  return n.toFixed(decimals)
}

function truncAddr(addr: string): string {
  if (addr.length <= 20) return addr
  return addr.slice(0, 12) + '...' + addr.slice(-6)
}

function formatDateTime(iso: string | null): string {
  if (!iso) return '—'
  const d = new Date(iso)
  return d.toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
}

function PnlValue({ value }: { value: string }) {
  const n = parseFloat(value)
  const color = n > 0 ? '#22c55e' : n < 0 ? '#ef4444' : 'var(--ink-subtle)'
  const prefix = n > 0 ? '+' : ''
  return (
    <span style={{ color, fontFamily: "'Chakra Petch', sans-serif" }} className="font-bold">
      {prefix}{formatNum(value, 4)}
    </span>
  )
}

export default function TraderPage() {
  const { address: paramAddr } = useParams<{ address: string }>()
  const navigate = useNavigate()
  const walletAddr = useWalletStore((s) => s.address)
  const [searchInput, setSearchInput] = useState('')

  const traderAddr = paramAddr || ''

  const traderQuery = useQuery({
    queryKey: ['trader-profile', traderAddr],
    queryFn: () => getTrader(traderAddr),
    enabled: !!traderAddr,
    refetchInterval: 30_000,
    retry: false,
  })

  const tradesQuery = useQuery({
    queryKey: ['trader-trades', traderAddr],
    queryFn: () => getTraderTrades(traderAddr, 100),
    enabled: !!traderAddr,
    refetchInterval: 15_000,
  })

  const positionsQuery = useQuery({
    queryKey: ['trader-positions', traderAddr],
    queryFn: () => getTraderPositions(traderAddr),
    enabled: !!traderAddr,
    refetchInterval: 30_000,
  })

  const trader = traderQuery.data
  const isOwnProfile = walletAddr && walletAddr === traderAddr

  const handleSearch = () => {
    const addr = searchInput.trim()
    if (addr) {
      sounds.playButtonPress()
      navigate(`/trader/${addr}`)
      setSearchInput('')
    }
  }

  return (
    <div className="space-y-4">
      <h1
        className="text-lg font-bold uppercase tracking-wider"
        style={{ color: 'var(--ink)', fontFamily: "'Chakra Petch', sans-serif" }}
      >
        Trader Profile
      </h1>

      {/* Search / My Profile */}
      <div className="shell-panel flex flex-col sm:flex-row gap-2">
        <div className="flex gap-2 flex-1">
          <input
            type="text"
            className="input-neo flex-1"
            placeholder="Search trader address..."
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
          />
          <button className="btn-primary !px-4 !py-1.5 !text-xs" onClick={handleSearch}>
            Search
          </button>
        </div>
        {walletAddr && (
          <Link
            to={`/trader/${walletAddr}`}
            onClick={() => sounds.playButtonPress()}
            className="btn-muted !px-4 !py-1.5 !text-xs text-center"
          >
            My Profile
          </Link>
        )}
      </div>

      {!traderAddr && (
        <div className="shell-panel-strong text-center py-12">
          <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
            Enter a trader address above or connect your wallet to view your profile.
          </p>
        </div>
      )}

      {traderAddr && traderQuery.isLoading && (
        <div className="flex items-center justify-center py-12 gap-2" style={{ color: 'var(--ink-subtle)' }}>
          <Spinner /> Loading trader profile...
        </div>
      )}

      {traderAddr && traderQuery.isError && (
        <div className="shell-panel-strong text-center py-12">
          <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
            Trader not found. They may not have traded yet.
          </p>
        </div>
      )}

      {trader && (
        <>
          {/* Profile Header */}
          <div className="shell-panel">
            <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 mb-3">
              <div>
                <p className="text-sm font-mono" style={{ color: 'var(--ink)' }}>
                  {truncAddr(trader.address)}
                  {isOwnProfile && (
                    <span className="ml-2 text-[10px] uppercase tracking-wider px-2 py-0.5 border border-white/30 rounded-sm" style={{ color: 'var(--accent)' }}>
                      You
                    </span>
                  )}
                </p>
              </div>
              {trader.tier_name && (
                <span className="text-[10px] uppercase tracking-wider px-2 py-1 border border-white/20 rounded-sm" style={{ color: 'var(--ink-subtle)' }}>
                  Tier: {trader.tier_name}
                </span>
              )}
            </div>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              <StatCard label="Total Trades" value={trader.total_trades.toLocaleString()} />
              <StatCard label="Total Volume" value={formatNum(trader.total_volume)} />
              <StatCard label="First Trade" value={formatDateTime(trader.first_trade_at)} />
              <StatCard label="Last Trade" value={formatDateTime(trader.last_trade_at)} />
            </div>
          </div>

          {/* P&L Summary */}
          <div className="shell-panel-strong">
            <h3
              className="text-sm font-semibold uppercase tracking-wide mb-3"
              style={{ color: 'var(--ink)', fontFamily: "'Chakra Petch', sans-serif" }}
            >
              P&L Summary
            </h3>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              <div className="p-3 border border-white/10 rounded-sm" style={{ background: 'var(--panel-bg)' }}>
                <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>Total Realized P&L</p>
                <PnlValue value={trader.total_realized_pnl} />
              </div>
              <div className="p-3 border border-white/10 rounded-sm" style={{ background: 'var(--panel-bg)' }}>
                <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>Best Trade</p>
                <PnlValue value={trader.best_trade_pnl} />
              </div>
              <div className="p-3 border border-white/10 rounded-sm" style={{ background: 'var(--panel-bg)' }}>
                <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>Worst Trade</p>
                <PnlValue value={trader.worst_trade_pnl} />
              </div>
              <div className="p-3 border border-white/10 rounded-sm" style={{ background: 'var(--panel-bg)' }}>
                <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>Total Fees Paid</p>
                <p className="text-sm font-bold" style={{ color: 'var(--ink)', fontFamily: "'Chakra Petch', sans-serif" }}>
                  {formatNum(trader.total_fees_paid)}
                </p>
              </div>
            </div>
          </div>

          {/* Open Positions */}
          <div className="shell-panel-strong">
            <h3
              className="text-sm font-semibold uppercase tracking-wide mb-3"
              style={{ color: 'var(--ink)', fontFamily: "'Chakra Petch', sans-serif" }}
            >
              Positions
            </h3>
            {positionsQuery.isLoading && (
              <div className="flex items-center justify-center py-6 gap-2" style={{ color: 'var(--ink-subtle)' }}>
                <Spinner /> Loading positions...
              </div>
            )}
            {positionsQuery.data && positionsQuery.data.length === 0 && (
              <p className="text-center py-6 text-sm" style={{ color: 'var(--ink-dim)' }}>No positions</p>
            )}
            {positionsQuery.data && positionsQuery.data.length > 0 && (
              <div className="overflow-x-auto">
                <table className="w-full text-xs">
                  <thead>
                    <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
                      <th className="text-left py-2 px-2 font-medium uppercase tracking-wider">Pair</th>
                      <th className="text-right py-2 px-2 font-medium uppercase tracking-wider">Net Position</th>
                      <th className="text-right py-2 px-2 font-medium uppercase tracking-wider">Avg Entry</th>
                      <th className="text-right py-2 px-2 font-medium uppercase tracking-wider">Cost Basis</th>
                      <th className="text-right py-2 px-2 font-medium uppercase tracking-wider">Realized P&L</th>
                      <th className="text-right py-2 px-2 font-medium uppercase tracking-wider">Trades</th>
                    </tr>
                  </thead>
                  <tbody>
                    {positionsQuery.data.map((pos: IndexerPosition) => (
                      <tr key={pos.pair_address} className="border-b border-white/5 hover:bg-white/5 transition-colors">
                        <td className="py-1.5 px-2 font-medium" style={{ color: 'var(--ink)' }}>
                          {pos.asset_0_symbol}/{pos.asset_1_symbol}
                        </td>
                        <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink)' }}>{formatNum(pos.net_position_quote, 4)}</td>
                        <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>{formatNum(pos.avg_entry_price, 6)}</td>
                        <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>{formatNum(pos.total_cost_base)}</td>
                        <td className="py-1.5 px-2 text-right"><PnlValue value={pos.realized_pnl} /></td>
                        <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>{pos.trade_count}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>

          {/* Trade History */}
          <div className="shell-panel-strong">
            <h3
              className="text-sm font-semibold uppercase tracking-wide mb-3"
              style={{ color: 'var(--ink)', fontFamily: "'Chakra Petch', sans-serif" }}
            >
              Trade History
            </h3>
            {tradesQuery.isLoading && (
              <div className="flex items-center justify-center py-6 gap-2" style={{ color: 'var(--ink-subtle)' }}>
                <Spinner /> Loading trades...
              </div>
            )}
            {tradesQuery.data && tradesQuery.data.length === 0 && (
              <p className="text-center py-6 text-sm" style={{ color: 'var(--ink-dim)' }}>No trades</p>
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
                    {tradesQuery.data.map((t: IndexerTrade) => (
                      <tr key={t.id} className="border-b border-white/5 hover:bg-white/5 transition-colors">
                        <td className="py-1.5 px-2" style={{ color: 'var(--ink-subtle)' }}>{formatDateTime(t.block_timestamp)}</td>
                        <td className="py-1.5 px-2 font-medium" style={{ color: 'var(--ink)' }}>
                          {t.offer_asset} → {t.ask_asset}
                        </td>
                        <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink)' }}>{formatNum(t.offer_amount)}</td>
                        <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink)' }}>{formatNum(t.return_amount)}</td>
                        <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>{formatNum(t.price, 6)}</td>
                        <td className="py-1.5 px-2" style={{ color: 'var(--ink-dim)' }}>{t.tx_hash.slice(0, 8)}...</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  )
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="p-3 border border-white/10 rounded-sm" style={{ background: 'var(--panel-bg)' }}>
      <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>{label}</p>
      <p className="text-sm font-bold" style={{ color: 'var(--ink)', fontFamily: "'Chakra Petch', sans-serif" }}>
        {value}
      </p>
    </div>
  )
}
