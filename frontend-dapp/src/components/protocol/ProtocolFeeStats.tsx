import { type UseQueryResult } from '@tanstack/react-query'
import { StatBox, RetryError } from '@/components/ui'
import { formatProtocolPct, formatProtocolUsd } from '@/utils/formatProtocolStats'
import {
  PROTOCOL_FEES_24H_CHG_LABEL,
  PROTOCOL_FEES_24H_LABEL,
  PROTOCOL_FEES_30D_CHG_LABEL,
  PROTOCOL_FEES_30D_LABEL,
  PROTOCOL_FEES_7D_CHG_LABEL,
  PROTOCOL_FEES_7D_LABEL,
  TRAILING_24H_FEES_CHG_TITLE,
  TRAILING_24H_FEES_TITLE,
  TRAILING_30D_FEES_CHG_TITLE,
  TRAILING_30D_FEES_TITLE,
  TRAILING_7D_FEES_CHG_TITLE,
  TRAILING_7D_FEES_TITLE,
} from '@/utils/trailingWindowCopy'
import type { IndexerOverview, ProtocolFeeSourceKey, ProtocolFeesResponse } from '@/types'

const SOURCE_LABEL: Record<ProtocolFeeSourceKey, string> = {
  wrap: 'Wrap',
  unwrap: 'Unwrap',
  swap_amm: 'AMM swap',
  book_take: 'Book take',
  limit_place: 'Limit place',
}

const SOURCE_ORDER: ProtocolFeeSourceKey[] = [
  'wrap',
  'unwrap',
  'swap_amm',
  'book_take',
  'limit_place',
]

function sourceLabel(source: string): string {
  if (source in SOURCE_LABEL) return SOURCE_LABEL[source as ProtocolFeeSourceKey]
  return source
}

interface ProtocolFeeStatsProps {
  overviewQuery: UseQueryResult<IndexerOverview>
  feesQuery: UseQueryResult<ProtocolFeesResponse>
}

function overviewHasFeeFields(o: IndexerOverview | undefined): boolean {
  if (!o) return false
  return (
    o.total_fees_24h_usd !== undefined ||
    o.total_fees_7d_usd !== undefined ||
    o.total_fees_30d_usd !== undefined
  )
}

export function ProtocolFeeStats({ overviewQuery, feesQuery }: ProtocolFeeStatsProps) {
  const overview = overviewQuery.data
  const fees = feesQuery.data
  const showPanel = overviewHasFeeFields(overview) || feesQuery.isSuccess
  if (!showPanel) return null

  const loading = overviewQuery.isLoading
  const wrapConfigured = fees?.wrap_mapper_configured === true

  const sources = (fees?.by_source ?? [])
    .filter((row) => {
      if (row.source !== 'wrap' && row.source !== 'unwrap') return true
      return wrapConfigured
    })
    .filter((row) => row.event_count !== 0 || row.amount_usd !== '0')
    .sort((a, b) => {
      const ia = SOURCE_ORDER.indexOf(a.source as ProtocolFeeSourceKey)
      const ib = SOURCE_ORDER.indexOf(b.source as ProtocolFeeSourceKey)
      return (ia === -1 ? 99 : ia) - (ib === -1 ? 99 : ib)
    })

  return (
    <div className="shell-panel" data-testid="protocol-fee-stats">
      <h2 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
        Protocol fees
      </h2>
      <p className="text-xs mb-3 max-w-2xl" style={{ color: 'var(--ink-dim)' }}>
        Treasury inflows indexed from swaps, book takes, limit places, and wrap. Reference only — not a CMM
        balance.
      </p>
      {overviewQuery.isError && (
        <RetryError message="Failed to load fee totals" onRetry={() => void overviewQuery.refetch()} />
      )}
      <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
        <div data-testid="protocol-stat-fees-24h">
          <StatBox
            label={PROTOCOL_FEES_24H_LABEL}
            title={TRAILING_24H_FEES_TITLE}
            value={formatProtocolUsd(overview?.total_fees_24h_usd)}
            loading={loading}
          />
        </div>
        <div data-testid="protocol-stat-fees-24h-chg">
          <StatBox
            label={PROTOCOL_FEES_24H_CHG_LABEL}
            title={TRAILING_24H_FEES_CHG_TITLE}
            value={formatProtocolPct(overview?.fees_change_24h_pct)}
            loading={loading}
          />
        </div>
        <div data-testid="protocol-stat-fees-7d">
          <StatBox
            label={PROTOCOL_FEES_7D_LABEL}
            title={TRAILING_7D_FEES_TITLE}
            value={formatProtocolUsd(overview?.total_fees_7d_usd)}
            loading={loading}
          />
        </div>
        <div data-testid="protocol-stat-fees-7d-chg">
          <StatBox
            label={PROTOCOL_FEES_7D_CHG_LABEL}
            title={TRAILING_7D_FEES_CHG_TITLE}
            value={formatProtocolPct(overview?.fees_change_7d_pct)}
            loading={loading}
          />
        </div>
        <div data-testid="protocol-stat-fees-30d">
          <StatBox
            label={PROTOCOL_FEES_30D_LABEL}
            title={TRAILING_30D_FEES_TITLE}
            value={formatProtocolUsd(overview?.total_fees_30d_usd)}
            loading={loading}
          />
        </div>
        <div data-testid="protocol-stat-fees-30d-chg">
          <StatBox
            label={PROTOCOL_FEES_30D_CHG_LABEL}
            title={TRAILING_30D_FEES_CHG_TITLE}
            value={formatProtocolPct(overview?.fees_change_30d_pct)}
            loading={loading}
          />
        </div>
      </div>

      {feesQuery.isError && (
        <div className="mt-3">
          <RetryError message="Failed to load fee breakdown" onRetry={() => void feesQuery.refetch()} />
        </div>
      )}

      {sources.length > 0 && (
        <div className="mt-4" data-testid="protocol-fees-by-source">
          <h3 className="text-xs font-semibold uppercase tracking-wide mb-2" style={{ color: 'var(--ink-dim)' }}>
            Source
          </h3>
          <table className="w-full text-sm">
            <tbody>
              {sources.map((row) => (
                <tr key={row.source}>
                  <td className="py-1 pr-3" style={{ color: 'var(--ink)' }}>
                    {sourceLabel(row.source)}
                  </td>
                  <td className="py-1 text-right tabular-nums" style={{ color: 'var(--ink)' }}>
                    {formatProtocolUsd(row.amount_usd)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {(fees?.by_token?.length ?? 0) > 0 && (
        <div className="mt-4" data-testid="protocol-fees-by-token">
          <h3 className="text-xs font-semibold uppercase tracking-wide mb-2" style={{ color: 'var(--ink-dim)' }}>
            Tokens
          </h3>
          <table className="w-full text-sm">
            <tbody>
              {(fees?.by_token ?? []).map((row) => (
                <tr key={`${row.asset_id ?? 'other'}-${row.symbol}`}>
                  <td className="py-1 pr-3" style={{ color: 'var(--ink)' }}>
                    {row.symbol}
                    {row.amount_human != null && !row.is_other ? (
                      <span className="ml-2 text-xs" style={{ color: 'var(--ink-dim)' }}>
                        {row.amount_human}
                      </span>
                    ) : null}
                  </td>
                  <td className="py-1 text-right tabular-nums" style={{ color: 'var(--ink)' }}>
                    {formatProtocolUsd(row.amount_usd)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
