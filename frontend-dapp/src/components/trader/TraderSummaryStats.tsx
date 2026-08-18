import { AddressRow } from '@/components/ui/AddressRow'
import { StatBox } from '@/components/ui/StatBox'
import { formatNum } from '@/utils/formatAmount'
import { formatIndexedVolumeUsd } from '@/utils/chartsOverviewStats'
import { formatDateTime } from '@/utils/formatDate'
import type { IndexerTrader } from '@/types'
import { PnlValue } from './PnlValue'

export type TraderSummaryStatsProps = {
  trader: IndexerTrader
  isOwnProfile?: boolean
  addressRowTestId?: string
}

/** Indexer trader profile header + aggregate stats (GitLab #212 shared with `/trader`). */
export function TraderSummaryStats({ trader, isOwnProfile, addressRowTestId }: TraderSummaryStatsProps) {
  return (
    <>
      <div className="shell-panel">
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 mb-3">
          <div>
            <p className="text-sm flex flex-wrap items-center gap-1" style={{ color: 'var(--ink)' }}>
              <AddressRow
                address={trader.address}
                startChars={12}
                endChars={6}
                copyAriaLabel="Copy trader address"
                explorerAriaLabel="View trader address on explorer"
                data-testid={addressRowTestId ?? 'trader-profile-address-row'}
              />
              {isOwnProfile && (
                <span className="badge-glass badge-glass-accent ml-2" style={{ color: 'var(--accent)' }}>
                  You
                </span>
              )}
            </p>
          </div>
          {trader.tier_name && (
            <span className="badge-glass" style={{ color: 'var(--ink-subtle)' }}>
              Tier: {trader.tier_name}
            </span>
          )}
        </div>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          <StatBox label="Total Trades" value={trader.total_trades.toLocaleString()} />
          <StatBox
            label="Total Volume (USD)"
            value={formatIndexedVolumeUsd(trader.total_volume_usd, trader.total_trades)}
            data-testid="trader-total-volume-usd"
          />
          <StatBox label="First Trade" value={formatDateTime(trader.first_trade_at)} />
          <StatBox label="Last Trade" value={formatDateTime(trader.last_trade_at)} />
        </div>
      </div>

      <div className="shell-panel-strong">
        <h3 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
          P&L Summary
        </h3>
        <p className="text-xs mb-3" style={{ color: 'var(--ink-dim)' }}>
          Indexer quote exposure · realized P&amp;L only — not on-chain balances or mark-to-market unrealized P&amp;L.
          See{' '}
          <a
            href="https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs/indexer-invariants.md"
            target="_blank"
            rel="noopener noreferrer"
            className="underline"
            style={{ color: 'var(--accent)' }}
          >
            docs/indexer-invariants.md
          </a>
          .
        </p>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          <div className="card-glass !p-3">
            <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>
              Total Realized P&L
            </p>
            <PnlValue value={trader.total_realized_pnl} />
          </div>
          <div className="card-glass !p-3">
            <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>
              Best Trade
            </p>
            <PnlValue value={trader.best_trade_pnl} />
          </div>
          <div className="card-glass !p-3">
            <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>
              Worst Trade
            </p>
            <PnlValue value={trader.worst_trade_pnl} />
          </div>
          <div className="card-glass !p-3">
            <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>
              Total Fees Paid
            </p>
            <p className="text-sm font-bold font-heading" style={{ color: 'var(--ink)' }}>
              {formatNum(trader.total_fees_paid)}
            </p>
          </div>
        </div>
      </div>
    </>
  )
}
