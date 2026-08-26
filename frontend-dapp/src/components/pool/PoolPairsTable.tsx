import type { IndexerPair } from '@/types'
import { indexerPairToPairInfo } from '@/types'
import { Link } from 'react-router-dom'
import { TokenDisplay, FeeDisplay, PairTokenLinks } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import { formatQuoteVolume24h } from '@/utils/formatAmount'
import { formatCreatedAtTitle, formatRelativeAge } from '@/utils/formatDate'
import { chartsPairHref } from '@/utils/chartsPairRoute'
import type { PoolColumnSort } from '@/utils/poolListQuery'
import { getPairListBadges, type PairListBadges } from '@/utils/pairListBadges'
import { POOL_VOL_HEADER_LABEL, POOL_VOL_HEADER_TITLE } from '@/utils/trailingWindowCopy'
import { PoolAdvancedManage } from '@/components/pool/PoolAdvancedManage'

export type PoolPairsTableProps = {
  pairs: IndexerPair[]
  factoryPairAddresses: Set<string>
  /** Active indexer column, or null when catalog default (no column caret active). */
  activeSort: PoolColumnSort | null
  order: 'asc' | 'desc'
  onSort: (sort: PoolColumnSort) => void
  expandedAddr: string | null
  onToggleManage: (pairAddress: string) => void
}

function sortAria(active: boolean, order: 'asc' | 'desc'): 'none' | 'ascending' | 'descending' {
  if (!active) return 'none'
  return order === 'asc' ? 'ascending' : 'descending'
}

function SortHeader({
  label,
  sortKey,
  activeSort,
  order,
  onSort,
  testId,
  align = 'left',
  title,
}: {
  label: string
  sortKey: PoolColumnSort
  activeSort: PoolColumnSort | null
  order: 'asc' | 'desc'
  onSort: (sort: PoolColumnSort) => void
  testId: string
  align?: 'left' | 'right'
  title?: string
}) {
  const active = activeSort === sortKey
  const caret = !active ? '↕' : order === 'asc' ? '↑' : '↓'
  const alignClass = align === 'right' ? 'text-right' : 'text-left'
  return (
    <th
      scope="col"
      className={`${alignClass} py-2 px-2 font-medium uppercase tracking-wider`}
      aria-sort={sortAria(active, order)}
      data-testid={`${testId}-th`}
    >
      <button
        type="button"
        className="inline-flex items-center gap-1 hover:opacity-80"
        style={{ color: 'inherit' }}
        title={title}
        onClick={() => {
          sounds.playButtonPress()
          onSort(sortKey)
        }}
        data-testid={testId}
      >
        <span>{label}</span>
        <span aria-hidden="true" className="text-[10px] leading-none" data-testid={`${testId}-caret`}>
          {caret}
        </span>
        <span className="sr-only">
          {active ? (order === 'asc' ? 'sorted ascending' : 'sorted descending') : 'not sorted'}
        </span>
      </button>
    </th>
  )
}

function FactoryMark({ badges }: { badges: PairListBadges }) {
  if (badges.isInFactoryRouterGraph) {
    return (
      <span
        className="text-[10px] font-semibold uppercase tracking-wide"
        style={{ color: 'var(--cyan)' }}
        title="Registered in the factory pair list."
        data-testid="pool-row-factory"
      >
        Factory
      </span>
    )
  }
  return (
    <span
      className="text-[10px] font-semibold uppercase tracking-wide"
      style={{ color: 'var(--ink-subtle)' }}
      title="Indexer row only — not in this session’s factory set."
      data-testid="pool-row-indexer-only"
    >
      Indexer
    </span>
  )
}

export function PoolPairsTable({
  pairs,
  factoryPairAddresses,
  activeSort,
  order,
  onSort,
  expandedAddr,
  onToggleManage,
}: PoolPairsTableProps) {
  return (
    <div className="overflow-x-auto" data-testid="pool-pairs-table-wrap">
      <table className="w-full text-xs min-w-[44rem]" data-testid="pool-pairs-table" aria-label="Liquidity pools">
        <thead>
          <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
            <SortHeader
              label="Pair"
              sortKey="symbol"
              activeSort={activeSort}
              order={order}
              onSort={onSort}
              testId="pool-sort-pair"
            />
            <SortHeader
              label={POOL_VOL_HEADER_LABEL}
              title={POOL_VOL_HEADER_TITLE}
              sortKey="volume_24h"
              activeSort={activeSort}
              order={order}
              onSort={onSort}
              testId="pool-sort-vol"
              align="right"
            />
            <SortHeader
              label="Fee"
              sortKey="fee"
              activeSort={activeSort}
              order={order}
              onSort={onSort}
              testId="pool-sort-fee"
              align="right"
            />
            <SortHeader
              label="Created"
              sortKey="created"
              activeSort={activeSort}
              order={order}
              onSort={onSort}
              testId="pool-sort-created"
            />
            <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">
              Source
            </th>
            <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
              <span className="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        {pairs.map((ip) => {
          const pair = indexerPairToPairInfo(ip)
          const badges = getPairListBadges({
            pairAddress: ip.pair_address,
            factoryPairAddresses,
          })
          const volumeLabel = formatQuoteVolume24h(ip.volume_quote_24h, ip.asset_1.decimals)
          const chartsHref = chartsPairHref(ip.pair_address)
          const expanded = expandedAddr === ip.pair_address
          return (
            <tbody key={ip.pair_address} data-testid="pool-pair-group">
              <PoolPairRows
                ip={ip}
                pair={pair}
                badges={badges}
                volumeLabel={volumeLabel}
                chartsHref={chartsHref}
                expanded={expanded}
                onToggleManage={onToggleManage}
              />
            </tbody>
          )
        })}
      </table>
    </div>
  )
}

function PoolPairRows({
  ip,
  pair,
  badges,
  volumeLabel,
  chartsHref,
  expanded,
  onToggleManage,
}: {
  ip: IndexerPair
  pair: ReturnType<typeof indexerPairToPairInfo>
  badges: PairListBadges
  volumeLabel: string | null
  chartsHref: string | null
  expanded: boolean
  onToggleManage: (pairAddress: string) => void
}) {
  return (
    <>
      <tr className="border-b border-white/5 hover:bg-white/5 transition-colors" data-testid="pool-pair-row">
        <td className="py-2 px-2 align-top">
          <div
            className="font-medium uppercase tracking-wide flex items-center gap-1 font-heading"
            style={{ color: 'var(--ink)' }}
          >
            <TokenDisplay info={pair.asset_infos[0]} size={16} />
            <span style={{ color: 'var(--ink-subtle)' }}>/</span>
            <TokenDisplay info={pair.asset_infos[1]} size={16} />
          </div>
          <PairTokenLinks pairAddress={pair.contract_addr} asset0={pair.asset_infos[0]} asset1={pair.asset_infos[1]} />
        </td>
        <td
          className="py-2 px-2 text-right font-mono align-top"
          style={{ color: 'var(--ink)' }}
          data-testid="pool-row-vol"
        >
          {volumeLabel || '—'}
        </td>
        <td className="py-2 px-2 text-right align-top" data-testid="pool-table-fee">
          {ip.fee_bps != null ? <FeeDisplay feeBps={ip.fee_bps} /> : '—'}
        </td>
        <td
          className="py-2 px-2 align-top whitespace-nowrap"
          style={{ color: 'var(--ink-subtle)' }}
          data-testid="pool-row-created"
          title={formatCreatedAtTitle(ip.created_at)}
        >
          {formatRelativeAge(ip.created_at)}
        </td>
        <td className="py-2 px-2 align-top">
          <FactoryMark badges={badges} />
          {ip.code_id_frozen ? (
            <span
              className="ml-2 text-[10px] font-semibold uppercase tracking-wide"
              style={{ color: 'var(--danger, #f87171)' }}
              title="Listed token code changed. Quotes can still appear; execute is blocked."
              data-testid="pool-row-code-id-frozen"
            >
              Frozen
            </span>
          ) : null}
        </td>
        <td className="py-2 px-2 text-right align-top whitespace-nowrap">
          {chartsHref ? (
            <Link
              to={chartsHref}
              className="underline mr-2"
              style={{ color: 'var(--cyan)' }}
              data-testid="pool-row-charts"
              onClick={() => sounds.playButtonPress()}
            >
              Charts
            </Link>
          ) : null}
          <button
            type="button"
            className="btn-muted !text-xs !px-2 !py-1"
            data-testid="pool-row-manage"
            aria-expanded={expanded}
            onClick={() => {
              sounds.playButtonPress()
              onToggleManage(ip.pair_address)
            }}
          >
            Manage
          </button>
        </td>
      </tr>
      {expanded ? (
        <tr data-testid="pool-row-manage-panel">
          <td colSpan={6} className="px-2 pb-4 pt-0">
            <PoolAdvancedManage
              pair={pair}
              volumeQuote24h={ip.volume_quote_24h}
              quoteDecimals={ip.asset_1.decimals}
              listBadges={badges}
            />
          </td>
        </tr>
      ) : null}
    </>
  )
}
