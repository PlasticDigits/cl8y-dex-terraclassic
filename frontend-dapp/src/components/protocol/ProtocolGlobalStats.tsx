import { type UseQueryResult } from '@tanstack/react-query'
import { StatBox, RetryError } from '@/components/ui'
import { formatProtocolCount, formatProtocolPct, formatProtocolUsd } from '@/utils/formatProtocolStats'
import {
  PROTOCOL_TRADES_24H_LABEL,
  PROTOCOL_VOLUME_24H_LABEL,
  PROTOCOL_VOLUME_7D_LABEL,
  PROTOCOL_VOLUME_30D_LABEL,
  TRAILING_24H_TRADES_TITLE,
  TRAILING_24H_VOLUME_TITLE,
  TRAILING_7D_VOLUME_TITLE,
  TRAILING_30D_VOLUME_TITLE,
} from '@/utils/trailingWindowCopy'
import type { IndexerOverview } from '@/types'

interface ProtocolGlobalStatsProps {
  overviewQuery: UseQueryResult<IndexerOverview>
}

const STATS: Array<{
  testId: string
  label: string
  title?: string
  value: (o: IndexerOverview | undefined) => string
}> = [
  {
    testId: 'protocol-stat-liquidity',
    label: 'Total liquidity',
    value: (o) => formatProtocolUsd(o?.total_liquidity_usd),
  },
  {
    testId: 'protocol-stat-liquidity-24h',
    label: '24h liquidity',
    value: (o) => formatProtocolPct(o?.liquidity_change_24h_pct),
  },
  {
    testId: 'protocol-stat-liquidity-30d',
    label: '30d liquidity',
    value: (o) => formatProtocolPct(o?.liquidity_change_30d_pct),
  },
  {
    testId: 'protocol-stat-volume-24h',
    label: PROTOCOL_VOLUME_24H_LABEL,
    title: TRAILING_24H_VOLUME_TITLE,
    value: (o) => formatProtocolUsd(o?.total_volume_24h_usd),
  },
  {
    testId: 'protocol-stat-volume-7d',
    label: PROTOCOL_VOLUME_7D_LABEL,
    title: TRAILING_7D_VOLUME_TITLE,
    value: (o) => formatProtocolUsd(o?.total_volume_7d_usd),
  },
  {
    testId: 'protocol-stat-volume-30d',
    label: PROTOCOL_VOLUME_30D_LABEL,
    title: TRAILING_30D_VOLUME_TITLE,
    value: (o) => formatProtocolUsd(o?.total_volume_30d_usd),
  },
  { testId: 'protocol-stat-tokens', label: 'Tokens', value: (o) => formatProtocolCount(o?.token_count) },
  {
    testId: 'protocol-stat-tokens-added',
    label: 'New tokens (30d)',
    value: (o) => formatProtocolCount(o?.tokens_added_30d),
  },
  {
    testId: 'protocol-stat-pairs-added',
    label: 'New pairs (30d)',
    value: (o) => formatProtocolCount(o?.pairs_added_30d),
  },
  {
    testId: 'protocol-stat-active-pairs',
    label: 'Active pairs',
    value: (o) => formatProtocolCount(o?.active_pairs_24h),
  },
  {
    testId: 'protocol-stat-trades-24h',
    label: PROTOCOL_TRADES_24H_LABEL,
    title: TRAILING_24H_TRADES_TITLE,
    value: (o) => formatProtocolCount(o?.total_trades_24h),
  },
]

export function ProtocolGlobalStats({ overviewQuery }: ProtocolGlobalStatsProps) {
  const overview = overviewQuery.data
  const loading = overviewQuery.isLoading

  return (
    <div className="shell-panel" data-testid="protocol-global-stats">
      <h2 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
        Global stats
      </h2>
      <p className="text-xs mb-3 max-w-2xl" style={{ color: 'var(--ink-dim)' }}>
        USD volume and pool TVL use the same USTC / LUNC / hub reference catalog. Liquidity is AMM reserves only (not
        book depth). 24h/30d liquidity is vs indexer snapshots, not on-chain genesis. TVL moves with LP, swaps, and
        reference prices.
      </p>
      {overviewQuery.isError && (
        <RetryError message="Failed to load global stats" onRetry={() => void overviewQuery.refetch()} />
      )}
      <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3">
        {STATS.map((stat) => (
          <div key={stat.testId} data-testid={stat.testId}>
            <StatBox
              variant="flat"
              label={stat.label}
              title={stat.title}
              value={stat.value(overview)}
              loading={loading}
            />
          </div>
        ))}
      </div>
    </div>
  )
}
