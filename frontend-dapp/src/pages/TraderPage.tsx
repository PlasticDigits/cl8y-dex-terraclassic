import { useState } from 'react'
import { useParams, useNavigate, Link } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { getTrader, getTraderTrades, getTraderPositions, INDEXER_URL } from '@/services/indexer/client'
import { StatBox, TradesTable, RetryError, Skeleton } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import { isValidTerraAddress } from '@/utils/constants'
import { isIndexerUnavailableError } from '@/utils/indexerErrors'
import { formatNum } from '@/utils/formatAmount'
import { shortenAddress } from '@/utils/tokenDisplay'
import { formatDateTime } from '@/utils/formatDate'
import type { IndexerPosition } from '@/types'

function PnlValue({ value }: { value: string }) {
  const n = parseFloat(value)
  const color = n > 0 ? 'var(--color-positive)' : n < 0 ? 'var(--color-negative)' : 'var(--ink-subtle)'
  const prefix = n > 0 ? '+' : ''
  return (
    <span style={{ color }} className="font-bold font-heading">
      {prefix}
      {formatNum(value, 4)}
    </span>
  )
}

export default function TraderPage() {
  const { address: paramAddr } = useParams<{ address?: string }>()
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

  const searchTrimmed = searchInput.trim()

  const handleSearch = () => {
    const addr = searchTrimmed
    if (addr && isValidTerraAddress(addr)) {
      sounds.playButtonPress()
      navigate(`/trader/${addr}`)
      setSearchInput('')
    }
  }

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-lg font-bold uppercase tracking-wider font-heading" style={{ color: 'var(--ink)' }}>
          Trader Profile
        </h1>
        <p className="text-sm mt-1" style={{ color: 'var(--ink-dim)' }}>
          Look up a wallet to review trading activity, positions, and P&amp;L.
        </p>
      </div>

      {/* Search / My Profile */}
      <div className="shell-panel flex flex-col sm:flex-row gap-2">
        <div className="flex gap-2 flex-1">
          <input
            type="text"
            className="input-neo flex-1"
            placeholder="Paste a trader wallet address"
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
          />
          <button className="btn-primary btn-cta !px-4 !py-1.5 !text-xs" onClick={handleSearch}>
            Search
          </button>
        </div>
        {walletAddr && (
          <Link
            to={`/trader/${walletAddr}`}
            onClick={() => sounds.playButtonPress()}
            className="btn-primary btn-cta !px-4 !py-1.5 !text-xs text-center no-underline self-start sm:self-auto"
          >
            My Profile
          </Link>
        )}
      </div>

      {!traderAddr && (
        <div className="shell-panel-strong text-center py-12">
          <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
            Search for a trader wallet above, or open your own profile once your wallet is connected.
          </p>
        </div>
      )}

      {traderAddr && traderQuery.isLoading && (
        <div className="shell-panel-strong space-y-3 py-6" aria-live="polite">
          <Skeleton height="1rem" width="40%" />
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
            {Array.from({ length: 4 }).map((_, i) => (
              <Skeleton key={i} height="3rem" />
            ))}
          </div>
        </div>
      )}

      {traderAddr && traderQuery.isError && isIndexerUnavailableError(traderQuery.error) && (
        <div className="alert-warning" role="alert">
          <p className="text-sm font-semibold uppercase tracking-wide font-heading" style={{ color: 'var(--ink)' }}>
            Indexer unavailable
          </p>
          <p className="text-xs mt-2 leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
            Trader profiles are loaded from the indexer at{' '}
            <code className="font-mono text-[11px] px-1 py-0.5 border border-white/20">{INDEXER_URL}</code>. Start the
            indexer or set{' '}
            <code className="font-mono text-[11px] px-1 py-0.5 border border-white/20">VITE_INDEXER_URL</code>.
          </p>
          <button
            type="button"
            className="btn-primary btn-cta !text-xs !px-4 !py-1.5 mt-3"
            onClick={() => void traderQuery.refetch()}
          >
            Retry
          </button>
        </div>
      )}

      {traderAddr && traderQuery.isError && !isIndexerUnavailableError(traderQuery.error) && (
        <RetryError
          message="Trader not found. They may not have traded yet."
          onRetry={() => void traderQuery.refetch()}
        />
      )}

      {trader && (
        <>
          {/* Profile Header */}
          <div className="shell-panel">
            <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 mb-3">
              <div>
                <p className="text-sm font-mono" style={{ color: 'var(--ink)' }}>
                  {shortenAddress(trader.address, 12, 6)}
                  {isOwnProfile && (
                    <span className="badge-neo badge-neo-accent ml-2" style={{ color: 'var(--accent)' }}>
                      You
                    </span>
                  )}
                </p>
              </div>
              {trader.tier_name && (
                <span className="badge-neo" style={{ color: 'var(--ink-subtle)' }}>
                  Tier: {trader.tier_name}
                </span>
              )}
            </div>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              <StatBox label="Total Trades" value={trader.total_trades.toLocaleString()} />
              <StatBox label="Total Volume" value={formatNum(trader.total_volume)} />
              <StatBox label="First Trade" value={formatDateTime(trader.first_trade_at)} />
              <StatBox label="Last Trade" value={formatDateTime(trader.last_trade_at)} />
            </div>
          </div>

          {/* P&L Summary */}
          <div className="shell-panel-strong">
            <h3
              className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading"
              style={{ color: 'var(--ink)' }}
            >
              P&L Summary
            </h3>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              <div className="card-neo !p-3">
                <p
                  className="text-[10px] uppercase tracking-wider font-medium mb-1"
                  style={{ color: 'var(--ink-dim)' }}
                >
                  Total Realized P&L
                </p>
                <PnlValue value={trader.total_realized_pnl} />
              </div>
              <div className="card-neo !p-3">
                <p
                  className="text-[10px] uppercase tracking-wider font-medium mb-1"
                  style={{ color: 'var(--ink-dim)' }}
                >
                  Best Trade
                </p>
                <PnlValue value={trader.best_trade_pnl} />
              </div>
              <div className="card-neo !p-3">
                <p
                  className="text-[10px] uppercase tracking-wider font-medium mb-1"
                  style={{ color: 'var(--ink-dim)' }}
                >
                  Worst Trade
                </p>
                <PnlValue value={trader.worst_trade_pnl} />
              </div>
              <div className="card-neo !p-3">
                <p
                  className="text-[10px] uppercase tracking-wider font-medium mb-1"
                  style={{ color: 'var(--ink-dim)' }}
                >
                  Total Fees Paid
                </p>
                <p className="text-sm font-bold font-heading" style={{ color: 'var(--ink)' }}>
                  {formatNum(trader.total_fees_paid)}
                </p>
              </div>
            </div>
          </div>

          {/* Open Positions */}
          <div className="shell-panel-strong">
            <h3
              className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading"
              style={{ color: 'var(--ink)' }}
            >
              Positions
            </h3>
            {positionsQuery.isLoading && (
              <div className="space-y-2 py-4" aria-live="polite">
                {Array.from({ length: 3 }).map((_, i) => (
                  <Skeleton key={i} height="1.5rem" />
                ))}
              </div>
            )}
            {positionsQuery.isError && (
              <RetryError message="Failed to load positions" onRetry={() => void positionsQuery.refetch()} />
            )}
            {positionsQuery.data && positionsQuery.data.length === 0 && (
              <p className="text-center py-6 text-sm" style={{ color: 'var(--ink-dim)' }}>
                No positions
              </p>
            )}
            {positionsQuery.data && positionsQuery.data.length > 0 && (
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
                    {positionsQuery.data.map((pos: IndexerPosition) => (
                      <tr key={pos.pair_address} className="border-b border-white/5 hover:bg-white/5 transition-colors">
                        <td className="py-1.5 px-2 font-medium" style={{ color: 'var(--ink)' }}>
                          {pos.asset_0_symbol}/{pos.asset_1_symbol}
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

          {/* Trade History */}
          <div className="shell-panel-strong">
            <h3
              className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading"
              style={{ color: 'var(--ink)' }}
            >
              Trade History
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
              <TradesTable trades={tradesQuery.data} formatTimeFn={formatDateTime} ariaLabel="Trade history" />
            )}
          </div>
        </>
      )}
    </div>
  )
}
