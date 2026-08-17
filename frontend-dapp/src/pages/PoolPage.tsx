import { useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useFeeDiscountRegistryStatus } from '@/hooks/useFeeDiscountRegistryStatus'
import { FeeDiscountRegistryWarning } from '@/components/feeDiscount/FeeDiscountRegistryWarning'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { FACTORY_CONTRACT_ADDRESS } from '@/utils/constants'
import { FACTORY_PAIRS_MAX_FOR_POOL_LIST } from '@/utils/pairListBadges'
import type { IndexerPair } from '@/types'
import { getPairs } from '@/services/indexer/client'
import { MarketDataServiceOutageBanner } from '@/components/common/MarketDataServiceOutageBanner'
import { isIndexerUnavailableError } from '@/utils/indexerErrors'
import { MARKET_DATA_SERVICE_OUTAGE_TITLE, POOL_MARKET_DATA_OUTAGE_LEAD } from '@/utils/marketDataServiceCopy'
import { RetryError, Skeleton } from '@/components/ui'
import { PoolLpHowto } from '@/components/pool/PoolLpHowto'
import { OneSidedAddCard } from '@/components/pool/OneSidedAddCard'
import { OneSidedWithdrawCard } from '@/components/pool/OneSidedWithdrawCard'
import { PoolPairsTable } from '@/components/pool/PoolPairsTable'
import {
  POOL_CATALOG_FETCH_LIMIT,
  POOL_PAGE_SIZE,
  catalogRankAndPaginate,
  defaultOrderForPoolSort,
  type PoolColumnSort,
  type PoolListMode,
} from '@/utils/poolListQuery'

export default function PoolPage() {
  const [q, setQ] = useState('')
  const [submittedQ, setSubmittedQ] = useState('')
  const [mode, setMode] = useState<PoolListMode>('catalog')
  const [columnSort, setColumnSort] = useState<PoolColumnSort>('volume_24h')
  const [order, setOrder] = useState<'asc' | 'desc'>('desc')
  const [page, setPage] = useState(0)
  const [expandedAddr, setExpandedAddr] = useState<string | null>(null)
  const { showFeeDiscountRegistryWarning } = useFeeDiscountRegistryStatus()

  const factoryPairsQuery = useQuery({
    queryKey: ['factoryPairsForPoolList', FACTORY_PAIRS_MAX_FOR_POOL_LIST] as const,
    queryFn: () => getAllPairsPaginated(FACTORY_PAIRS_MAX_FOR_POOL_LIST),
    staleTime: 60_000,
    enabled: !!FACTORY_CONTRACT_ADDRESS,
  })

  const factoryPairAddresses = useMemo(() => {
    const rows = factoryPairsQuery.data?.pairs ?? []
    return new Set(rows.map((p) => p.contract_addr))
  }, [factoryPairsQuery.data?.pairs])

  const searchQ = submittedQ.trim()
  const listMode: PoolListMode = searchQ ? 'search' : mode

  const pairsQuery = useQuery({
    queryKey: ['indexer-pairs-pool', listMode, searchQ, columnSort, order, page],
    queryFn: () => {
      if (listMode === 'catalog') {
        return getPairs({
          limit: POOL_CATALOG_FETCH_LIMIT,
          offset: 0,
          sort: 'volume_24h',
          order: 'desc',
        })
      }
      if (listMode === 'search') {
        return getPairs({
          limit: POOL_PAGE_SIZE,
          offset: page * POOL_PAGE_SIZE,
          q: searchQ,
          sort: 'relevance',
          order: 'desc',
        })
      }
      return getPairs({
        limit: POOL_PAGE_SIZE,
        offset: page * POOL_PAGE_SIZE,
        sort: columnSort,
        order,
      })
    },
    staleTime: 30_000,
  })

  const indexerPairs = useMemo(() => pairsQuery.data?.items ?? [], [pairsQuery.data?.items])

  const { visiblePairs, total } = useMemo(() => {
    if (listMode === 'catalog') {
      const ranked = catalogRankAndPaginate(indexerPairs, page, POOL_PAGE_SIZE)
      return { visiblePairs: ranked.pageItems, total: ranked.total }
    }
    return { visiblePairs: indexerPairs, total: pairsQuery.data?.total ?? indexerPairs.length }
  }, [listMode, indexerPairs, page, pairsQuery.data?.total])

  const totalPages = Math.max(1, Math.ceil(total / POOL_PAGE_SIZE))
  const canPrev = page > 0
  const canNext = (page + 1) * POOL_PAGE_SIZE < total
  const activeSort: PoolColumnSort | null = listMode === 'column' ? columnSort : null

  const submitSearch = () => {
    setPage(0)
    setExpandedAddr(null)
    setSubmittedQ(q)
    if (!q.trim()) setMode('catalog')
  }

  const onSort = (sort: PoolColumnSort) => {
    setMode('column')
    setPage(0)
    setExpandedAddr(null)
    if (columnSort === sort && mode === 'column') {
      setOrder((prev) => (prev === 'asc' ? 'desc' : 'asc'))
      return
    }
    setColumnSort(sort)
    setOrder(defaultOrderForPoolSort(sort))
  }

  return (
    <div className="max-w-6xl mx-auto">
      <PoolLpHowto />

      <div className="grid gap-4 mb-6 md:grid-cols-2">
        <OneSidedAddCard factoryPairs={factoryPairsQuery.data?.pairs ?? []} />
        <OneSidedWithdrawCard factoryPairs={factoryPairsQuery.data?.pairs ?? []} />
      </div>

      <div className="shell-panel mb-4" role="search" aria-label="Search pools">
        <label htmlFor="pool-search" className="label-glass mb-1 block">
          Search
        </label>
        <div className="flex flex-col sm:flex-row gap-2">
          <input
            id="pool-search"
            type="search"
            className="input-glass flex-1"
            placeholder="Symbol, address, denom…"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') submitSearch()
            }}
          />
          <button type="button" className="btn-muted shrink-0" onClick={submitSearch}>
            Search
          </button>
        </div>
      </div>

      {showFeeDiscountRegistryWarning && <FeeDiscountRegistryWarning testId="pool-fee-discount-registry-warning" />}

      {pairsQuery.isLoading && (
        <div className="space-y-4" aria-live="polite">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} height="3rem" />
          ))}
        </div>
      )}

      {pairsQuery.isError && isIndexerUnavailableError(pairsQuery.error) && (
        <MarketDataServiceOutageBanner
          testId="pool-market-data-outage-banner"
          title={MARKET_DATA_SERVICE_OUTAGE_TITLE}
          lead={POOL_MARKET_DATA_OUTAGE_LEAD}
          onRetry={() => void pairsQuery.refetch()}
        />
      )}

      {pairsQuery.isError && !isIndexerUnavailableError(pairsQuery.error) && (
        <RetryError
          message="Pool data is unavailable right now. Try again in a moment."
          onRetry={() => void pairsQuery.refetch()}
        />
      )}

      {!pairsQuery.isLoading && visiblePairs.length === 0 && !pairsQuery.isError && (
        <div className="shell-panel-strong py-8 text-center" style={{ color: 'var(--ink-dim)' }}>
          No liquidity pools match your filters.
        </div>
      )}

      {!pairsQuery.isLoading && visiblePairs.length > 0 && !pairsQuery.isError && (
        <PoolPairsTable
          pairs={visiblePairs as IndexerPair[]}
          factoryPairAddresses={factoryPairAddresses}
          activeSort={activeSort}
          order={order}
          onSort={onSort}
          expandedAddr={expandedAddr}
          onToggleManage={(addr) => setExpandedAddr((cur) => (cur === addr ? null : addr))}
        />
      )}

      {total > POOL_PAGE_SIZE && !pairsQuery.isLoading && !pairsQuery.isError && (
        <div className="flex items-center justify-between mt-6 pt-4 border-t border-white/10">
          <span className="text-xs uppercase tracking-wide" style={{ color: 'var(--ink-dim)' }}>
            Page {page + 1} of {totalPages}
          </span>
          <div className="flex gap-2">
            <button
              type="button"
              className="btn-muted !text-xs"
              disabled={!canPrev}
              onClick={() => {
                setPage((p) => Math.max(0, p - 1))
                setExpandedAddr(null)
              }}
            >
              Previous
            </button>
            <button
              type="button"
              className="btn-muted !text-xs"
              disabled={!canNext}
              onClick={() => {
                setPage((p) => p + 1)
                setExpandedAddr(null)
              }}
            >
              Next
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
