import { useQuery } from '@tanstack/react-query'
import { AddressRow } from '@/components/ui/AddressRow'
import { StatBox } from '@/components/ui/StatBox'
import { getOraclePrice } from '@/services/indexer/client'
import { formatIndexedVolumeUsd } from '@/utils/chartsOverviewStats'
import { formatDateTime } from '@/utils/formatDate'
import { formatSignedUsd, sumRealizedPnlUsd, TRADER_PNL_EM_DASH } from '@/utils/traderPositionDisplay'
import type { IndexerPosition, IndexerTrader } from '@/types'
import { PnlValue } from './PnlValue'

export type TraderSummaryStatsProps = {
  trader: IndexerTrader
  positions?: IndexerPosition[]
  isOwnProfile?: boolean
  addressRowTestId?: string
}

function parseOracleUsd(priceUsd: string | null | undefined): number | null {
  if (priceUsd == null || priceUsd === '') return null
  const n = Number(priceUsd)
  return Number.isFinite(n) && n > 0 ? n : null
}

/** Indexer trader profile header + aggregate stats (GitLab #212 / #551 / #553 shared with `/trader`). */
export function TraderSummaryStats({ trader, positions, isOwnProfile, addressRowTestId }: TraderSummaryStatsProps) {
  const ustcQuery = useQuery({
    queryKey: ['oracle-price', 'ustc'],
    queryFn: () => getOraclePrice('ustc'),
    staleTime: 60_000,
  })
  const luncQuery = useQuery({
    queryKey: ['oracle-price', 'lunc'],
    queryFn: () => getOraclePrice('lunc'),
    staleTime: 60_000,
  })

  const pnlUsd = sumRealizedPnlUsd(positions, {
    ustcUsd: parseOracleUsd(ustcQuery.data?.price_usd),
    luncUsd: parseOracleUsd(luncQuery.data?.price_usd),
  })
  const pnlLabel = pnlUsd.unpricedPairs > 0 && pnlUsd.pricedPairs > 0 ? 'Priced pairs only' : 'USD · realized only'

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
        <p className="text-xs mt-2" style={{ color: 'var(--ink-dim)' }}>
          Fees are not summed across pairs — each pair uses a different token.
        </p>
      </div>

      <div className="shell-panel-strong">
        <h3 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
          P&L Summary
        </h3>
        <p className="text-xs mb-3" style={{ color: 'var(--ink-dim)' }}>
          Per-pair figures use that pair&apos;s tokens. Cross-pair totals are USD or omitted when units differ. Realized
          only — not on-chain balances or mark-to-market. See{' '}
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
          <div className="card-glass !p-3" data-testid="trader-summary-realized-pnl">
            <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>
              Total Realized P&L
            </p>
            <p
              className="text-sm font-bold font-heading"
              style={{
                color:
                  pnlUsd.usd == null || pnlUsd.usd === 0
                    ? 'var(--ink-subtle)'
                    : pnlUsd.usd > 0
                      ? 'var(--color-positive)'
                      : 'var(--color-negative)',
              }}
            >
              {formatSignedUsd(pnlUsd.usd)}
            </p>
            <p className="text-[10px] mt-1" style={{ color: 'var(--ink-dim)' }}>
              {pnlLabel}
            </p>
          </div>
          <div className="card-glass !p-3" data-testid="trader-summary-best-trade">
            <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>
              Best Trade
            </p>
            <PnlValue value={null} />
            <p className="text-[10px] mt-1" style={{ color: 'var(--ink-dim)' }}>
              Not comparable across pairs
            </p>
          </div>
          <div className="card-glass !p-3" data-testid="trader-summary-worst-trade">
            <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>
              Worst Trade
            </p>
            <PnlValue value={null} />
            <p className="text-[10px] mt-1" style={{ color: 'var(--ink-dim)' }}>
              Not comparable across pairs
            </p>
          </div>
          <div className="card-glass !p-3" data-testid="trader-summary-fees">
            <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>
              Total Fees Paid
            </p>
            <p className="text-sm font-bold font-heading" style={{ color: 'var(--ink)' }}>
              {TRADER_PNL_EM_DASH}
            </p>
            <p className="text-[10px] mt-1" style={{ color: 'var(--ink-dim)' }}>
              Mixed tokens — not summed
            </p>
          </div>
        </div>
      </div>
    </>
  )
}
