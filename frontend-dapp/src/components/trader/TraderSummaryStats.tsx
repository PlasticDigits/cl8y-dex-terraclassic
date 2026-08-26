import { useQuery } from '@tanstack/react-query'
import { AddressRow } from '@/components/ui/AddressRow'
import { StatBox } from '@/components/ui/StatBox'
import { useProtocolHubPricesQuery } from '@/components/protocol/useProtocolHubPricesQuery'
import { getOraclePrice } from '@/services/indexer/client'
import { formatIndexedVolumeUsd } from '@/utils/chartsOverviewStats'
import { formatDateTime } from '@/utils/formatDate'
import {
  formatSignedUsd,
  sumRealizedPnlUsd,
  traderUsdMarksFromHub,
  TRADER_PNL_EM_DASH,
} from '@/utils/traderPositionDisplay'
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

/** Indexer trader profile header + aggregate stats (GitLab #212 / #551 / #553 / #560 shared with `/trader`). */
export function TraderSummaryStats({ trader, positions, isOwnProfile, addressRowTestId }: TraderSummaryStatsProps) {
  const hubQuery = useProtocolHubPricesQuery()
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

  const pnlUsd = sumRealizedPnlUsd(
    positions,
    traderUsdMarksFromHub(hubQuery.data, {
      ustcUsd: parseOracleUsd(ustcQuery.data?.price_usd),
      luncUsd: parseOracleUsd(luncQuery.data?.price_usd),
    })
  )
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
          <StatBox variant="flat" label="Total Trades" value={trader.total_trades.toLocaleString()} />
          <StatBox
            variant="flat"
            label="Total Volume (USD)"
            value={formatIndexedVolumeUsd(trader.total_volume_usd, trader.total_trades)}
            data-testid="trader-total-volume-usd"
          />
          <StatBox variant="flat" label="First Trade" value={formatDateTime(trader.first_trade_at)} />
          <StatBox variant="flat" label="Last Trade" value={formatDateTime(trader.last_trade_at)} />
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
          <StatBox
            variant="flat"
            label="Total Realized P&L"
            value={formatSignedUsd(pnlUsd.usd)}
            color={
              pnlUsd.usd == null || pnlUsd.usd === 0
                ? 'var(--ink-subtle)'
                : pnlUsd.usd > 0
                  ? 'var(--color-positive)'
                  : 'var(--color-negative)'
            }
            hint={pnlLabel}
            data-testid="trader-summary-realized-pnl"
          />
          <div className="stat-flat" data-testid="trader-summary-best-trade">
            <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>
              Best Trade
            </p>
            <PnlValue value={null} />
            <p className="text-[10px] mt-1" style={{ color: 'var(--ink-dim)' }}>
              Not comparable across pairs
            </p>
          </div>
          <div className="stat-flat" data-testid="trader-summary-worst-trade">
            <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>
              Worst Trade
            </p>
            <PnlValue value={null} />
            <p className="text-[10px] mt-1" style={{ color: 'var(--ink-dim)' }}>
              Not comparable across pairs
            </p>
          </div>
          <StatBox
            variant="flat"
            label="Total Fees Paid"
            value={TRADER_PNL_EM_DASH}
            hint="Mixed tokens — not summed"
            data-testid="trader-summary-fees"
          />
        </div>
      </div>
    </>
  )
}
