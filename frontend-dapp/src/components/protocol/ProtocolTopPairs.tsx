import { type UseQueryResult } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { RetryError, Skeleton } from '@/components/ui'
import { formatPairListVolumeUsd } from '@/utils/chartsOverviewStats'
import { formatProtocolUsd, formatVolumePerTvl } from '@/utils/formatProtocolStats'
import { chartsPairHref } from '@/utils/chartsPairRoute'
import {
  PROTOCOL_TOP_PAIRS_EMPTY,
  PROTOCOL_TOP_PAIRS_LEAD,
  PROTOCOL_TOP_PAIRS_LP_LABEL,
  PROTOCOL_TOP_PAIRS_LP_TITLE,
  PROTOCOL_TOP_PAIRS_PAIR_LABEL,
  PROTOCOL_TOP_PAIRS_RATIO_LABEL,
  PROTOCOL_TOP_PAIRS_RATIO_TITLE,
  PROTOCOL_TOP_PAIRS_TITLE,
  PROTOCOL_TOP_PAIRS_VOL_LABEL,
  TRAILING_30D_VOLUME_TITLE,
} from '@/utils/trailingWindowCopy'
import type { ProtocolTopPairItem, ProtocolTopPairsResponse } from '@/types'
import { isProtocolTopPairsUnavailable } from '@/components/protocol/useProtocolTopPairsQuery'

function pairLabel(item: ProtocolTopPairItem): string {
  const a = item.asset_0?.symbol?.trim() || '—'
  const b = item.asset_1?.symbol?.trim() || '—'
  return `${a} / ${b}`
}

function TopPairRow({ item, index }: { item: ProtocolTopPairItem; index: number }) {
  const href = chartsPairHref(item.pair_address)
  const label = pairLabel(item)
  const vol = formatPairListVolumeUsd(item.volume_usd_30d)
  const lp = formatProtocolUsd(item.liquidity_usd)
  const ratio = formatVolumePerTvl(item.volume_per_tvl)
  return (
    <tr data-testid={`protocol-top-pair-row-${index}`}>
      <td className="py-2 px-2" data-testid={`protocol-top-pair-pair-${index}`}>
        {href ? (
          <Link to={href} className="font-medium" style={{ color: 'var(--ink)' }}>
            {label}
          </Link>
        ) : (
          <span style={{ color: 'var(--ink)' }}>{label}</span>
        )}
      </td>
      <td className="py-2 px-2 text-right font-mono" data-testid={`protocol-top-pair-vol-${index}`}>
        {vol}
      </td>
      <td className="py-2 px-2 text-right font-mono" data-testid={`protocol-top-pair-lp-${index}`}>
        {lp}
      </td>
      <td className="py-2 px-2 text-right font-mono" data-testid={`protocol-top-pair-ratio-${index}`}>
        {ratio}
      </td>
    </tr>
  )
}

export function ProtocolTopPairs({ query }: { query: UseQueryResult<ProtocolTopPairsResponse, Error> }) {
  if (query.isError && isProtocolTopPairsUnavailable(query.error)) {
    return null
  }

  const rows = (query.data?.items ?? []).slice(0, 5)

  return (
    <section className="shell-panel" data-testid="protocol-top-pairs">
      <h2 className="text-sm font-semibold uppercase tracking-wide mb-1 font-heading" style={{ color: 'var(--ink)' }}>
        {PROTOCOL_TOP_PAIRS_TITLE}
      </h2>
      <p className="text-xs mb-3 max-w-2xl" style={{ color: 'var(--ink-dim)' }}>
        {PROTOCOL_TOP_PAIRS_LEAD}
      </p>
      {query.isLoading && (
        <div className="space-y-2 py-2" aria-live="polite">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} height="1.5rem" />
          ))}
        </div>
      )}
      {query.isError && <RetryError message="Failed to load top pairs" onRetry={() => void query.refetch()} />}
      {query.isSuccess && rows.length === 0 && (
        <p className="text-sm py-2" style={{ color: 'var(--ink-dim)' }} data-testid="protocol-top-pairs-empty">
          {PROTOCOL_TOP_PAIRS_EMPTY}
        </p>
      )}
      {query.isSuccess && rows.length > 0 && (
        <div className="overflow-x-auto">
          <table className="w-full text-xs" aria-label={PROTOCOL_TOP_PAIRS_TITLE}>
            <thead>
              <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
                <th className="text-left py-2 px-2 font-medium uppercase tracking-wider">
                  {PROTOCOL_TOP_PAIRS_PAIR_LABEL}
                </th>
                <th
                  className="text-right py-2 px-2 font-medium uppercase tracking-wider"
                  title={TRAILING_30D_VOLUME_TITLE}
                >
                  {PROTOCOL_TOP_PAIRS_VOL_LABEL}
                </th>
                <th
                  className="text-right py-2 px-2 font-medium uppercase tracking-wider"
                  title={PROTOCOL_TOP_PAIRS_LP_TITLE}
                >
                  {PROTOCOL_TOP_PAIRS_LP_LABEL}
                </th>
                <th
                  className="text-right py-2 px-2 font-medium uppercase tracking-wider"
                  title={PROTOCOL_TOP_PAIRS_RATIO_TITLE}
                >
                  {PROTOCOL_TOP_PAIRS_RATIO_LABEL}
                </th>
              </tr>
            </thead>
            <tbody>
              {rows.map((item, index) => (
                <TopPairRow key={`${item.pair_address}-${index}`} item={item} index={index} />
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  )
}
