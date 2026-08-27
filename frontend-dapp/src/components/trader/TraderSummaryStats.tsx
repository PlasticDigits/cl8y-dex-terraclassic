import { TraderIdentity } from '@/components/trader/TraderIdentity'
import { AddressRow } from '@/components/ui/AddressRow'
import { StatBox } from '@/components/ui/StatBox'
import { useTraderUsdMarks } from '@/hooks/useTraderUsdMarks'
import { formatIndexedVolumeUsd } from '@/utils/chartsOverviewStats'
import { formatDateTime } from '@/utils/formatDate'
import {
  formatSignedUsd,
  sumRealizedPnlUsd,
  sumUnrealizedPnlUsd,
  TRADER_PNL_EM_DASH,
} from '@/utils/traderPositionDisplay'
import type { IndexerPosition, IndexerTrader } from '@/types'
import { TRADER_ADDR_END_CHARS, TRADER_ADDR_START_CHARS } from '@/utils/tokenDisplay'
import { PnlValue } from './PnlValue'

export type TraderSummaryStatsProps = {
  trader: IndexerTrader
  positions?: IndexerPosition[]
  isOwnProfile?: boolean
  addressRowTestId?: string
}

function usdHint(summary: { pricedPairs: number; unpricedPairs: number }): string {
  return summary.unpricedPairs > 0 && summary.pricedPairs > 0 ? 'Priced pairs only' : 'USD · hub mark'
}

/** Indexer trader profile header + aggregate stats (GitLab #212 / #551 / #553 / #560 / #675 shared with `/trader`). */
export function TraderSummaryStats({ trader, positions, isOwnProfile, addressRowTestId }: TraderSummaryStatsProps) {
  const marks = useTraderUsdMarks()
  const pnlUsd = sumRealizedPnlUsd(positions, marks)
  const unrealizedUsd = sumUnrealizedPnlUsd(positions, marks)

  return (
    <>
      <div className="shell-panel">
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 mb-3">
          <div>
            <p className="text-sm flex flex-wrap items-center gap-2" style={{ color: 'var(--ink)' }}>
              <TraderIdentity address={trader.address} size={36} data-testid="trader-profile-identity">
                <AddressRow
                  address={trader.address}
                  startChars={TRADER_ADDR_START_CHARS}
                  endChars={TRADER_ADDR_END_CHARS}
                  copyAriaLabel="Copy trader address"
                  explorerAriaLabel="View trader address on explorer"
                  data-testid={addressRowTestId ?? 'trader-profile-address-row'}
                />
              </TraderIdentity>
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
          is closed quote sales. Unrealized is the current hub mark of remaining quote minus on-DEX cost — not wallet
          balances. See{' '}
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
        <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
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
            hint={usdHint(pnlUsd)}
            data-testid="trader-summary-realized-pnl"
          />
          <StatBox
            variant="flat"
            label="Total Unrealized P&L"
            value={formatSignedUsd(unrealizedUsd.usd)}
            color={
              unrealizedUsd.usd == null || unrealizedUsd.usd === 0
                ? 'var(--ink-subtle)'
                : unrealizedUsd.usd > 0
                  ? 'var(--color-positive)'
                  : 'var(--color-negative)'
            }
            hint={
              unrealizedUsd.noCostBasisPairs > 0 ? 'On-DEX cost only · some rows have no basis' : usdHint(unrealizedUsd)
            }
            data-testid="trader-summary-unrealized-pnl"
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
