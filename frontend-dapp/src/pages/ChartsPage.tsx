import { useState, useDeferredValue, useEffect, useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router-dom'
import { getOverview, getPairs, getPair, getPairStats, getTrades, getLeaderboard } from '@/services/indexer/client'
import { MarketDataServiceOutageBanner } from '@/components/common/MarketDataServiceOutageBanner'
import { CHARTS_MARKET_DATA_OUTAGE_LEAD, MARKET_DATA_SERVICE_OUTAGE_TITLE } from '@/utils/marketDataServiceCopy'
import { detectMarketDataOutage } from '@/utils/marketDataOutage'
import PriceChart from '@/components/charts/PriceChart'
import { StatBox, TradesTable, RetryError, Skeleton, MenuSelect, type MenuSelectOption } from '@/components/ui'
import { sounds } from '@/lib/sounds'
import { PnlValue } from '@/components/trader/PnlValue'
import { formatNum } from '@/utils/formatAmount'
import { indexerPairMenuLabel, indexerPairsToMenuSelectOptions } from '@/utils/pairMenuOptions'
import { shortenAddress } from '@/utils/tokenDisplay'
import { formatTime, formatTimeFromUnixSeconds } from '@/utils/formatDate'
import { getTwapPrices, getOracleInfo } from '@/services/terraclassic/oracle'
import type { IndexerPair, IndexerPairSort, IndexerTrader } from '@/types'

const PAIR_PAGE_SIZE = 50

const TWAP_WINDOWS = [
  { label: '5m', seconds: 300 },
  { label: '1h', seconds: 3600 },
  { label: '24h', seconds: 86400 },
]

const LEADERBOARD_TABS = [
  { key: 'total_volume', label: 'Volume' },
  { key: 'best_trade_pnl', label: 'Best Trade' },
  { key: 'total_realized_pnl', label: 'Most Profit' },
  { key: 'worst_trade_pnl', label: 'Most Loss' },
] as const

const CHARTS_PAIR_SORT_OPTIONS: MenuSelectOption[] = [
  { value: 'volume_24h', label: '24h volume' },
  { value: 'symbol', label: 'Name (A–Z)' },
  { value: 'fee', label: 'Fee' },
  { value: 'created', label: 'Created' },
  { value: 'id', label: 'Pair ID' },
]

const ORDER_OPTIONS: MenuSelectOption[] = [
  { value: 'asc', label: 'Ascending' },
  { value: 'desc', label: 'Descending' },
]

export default function ChartsPage() {
  const [selectedPairAddr, setSelectedPairAddr] = useState<string>('')
  const [pairSearch, setPairSearch] = useState('')
  const [pairSort, setPairSort] = useState<IndexerPairSort>('volume_24h')
  const [pairOrder, setPairOrder] = useState<'asc' | 'desc'>('desc')
  const [pairPage, setPairPage] = useState(0)
  const [leaderboardSort, setLeaderboardSort] = useState<string>('total_volume')
  const deferredPairSearch = useDeferredValue(pairSearch.trim())

  useEffect(() => {
    setPairPage(0)
  }, [deferredPairSearch])

  const overviewQuery = useQuery({
    queryKey: ['indexer-overview'],
    queryFn: getOverview,
    refetchInterval: 30_000,
  })

  const pairsQuery = useQuery({
    queryKey: ['indexer-pairs', deferredPairSearch, pairSort, pairOrder, pairPage],
    queryFn: () =>
      getPairs({
        limit: PAIR_PAGE_SIZE,
        offset: pairPage * PAIR_PAGE_SIZE,
        q: deferredPairSearch || undefined,
        sort: pairSort,
        order: pairOrder,
      }),
    staleTime: 60_000,
  })

  const pairItems = pairsQuery.data?.items
  const pairTotal = pairsQuery.data?.total ?? 0
  const pairTotalPages = Math.max(1, Math.ceil(pairTotal / PAIR_PAGE_SIZE))
  const canPairPrev = pairPage > 0
  const canPairNext = (pairPage + 1) * PAIR_PAGE_SIZE < pairTotal

  const needsPairFetch =
    !!selectedPairAddr && !(pairItems ?? []).some((p: IndexerPair) => p.pair_address === selectedPairAddr)

  const selectedPairQuery = useQuery({
    queryKey: ['indexer-pair-one', selectedPairAddr],
    queryFn: () => getPair(selectedPairAddr),
    enabled: needsPairFetch,
    staleTime: 60_000,
    retry: false,
  })

  useEffect(() => {
    if (selectedPairQuery.isError) {
      setSelectedPairAddr('')
    }
  }, [selectedPairQuery.isError])

  const pairOptions = useMemo(() => {
    const list = [...(pairItems ?? [])]
    const extra = selectedPairQuery.data
    if (extra && !list.some((p) => p.pair_address === extra.pair_address)) {
      list.unshift(extra)
    }
    return list
  }, [pairItems, selectedPairQuery.data])

  const activePairAddr = selectedPairAddr || pairOptions[0]?.pair_address || ''
  const activePair = pairOptions.find((p: IndexerPair) => p.pair_address === activePairAddr)

  const pairMenuOptions = useMemo(
    () => indexerPairsToMenuSelectOptions(pairOptions, { variant: 'compact' }),
    [pairOptions]
  )

  useEffect(() => {
    if (pairOptions.length === 0) return
    if (!selectedPairAddr) return
    if (pairOptions.some((p) => p.pair_address === selectedPairAddr)) return
    if (needsPairFetch && selectedPairQuery.isLoading) return
    setSelectedPairAddr(pairOptions[0].pair_address)
  }, [pairOptions, selectedPairAddr, needsPairFetch, selectedPairQuery.isLoading])

  const statsQuery = useQuery({
    queryKey: ['pair-stats', activePairAddr],
    queryFn: () => getPairStats(activePairAddr),
    enabled: !!activePairAddr,
    refetchInterval: 30_000,
  })

  const tradesQuery = useQuery({
    queryKey: ['pair-trades', activePairAddr],
    queryFn: () => getTrades(activePairAddr, 50),
    enabled: !!activePairAddr,
    refetchInterval: 15_000,
  })

  const leaderboardQuery = useQuery({
    queryKey: ['leaderboard', leaderboardSort],
    queryFn: () => getLeaderboard(leaderboardSort, 20),
    refetchInterval: 30_000,
  })

  const twapQuery = useQuery({
    queryKey: ['twap-prices', activePairAddr],
    queryFn: () => getTwapPrices(activePairAddr, TWAP_WINDOWS),
    enabled: !!activePairAddr,
    staleTime: 30_000,
    refetchInterval: 60_000,
    retry: false,
  })

  const oracleInfoQuery = useQuery({
    queryKey: ['oracle-info', activePairAddr],
    queryFn: () => getOracleInfo(activePairAddr),
    enabled: !!activePairAddr,
    staleTime: 60_000,
    retry: false,
  })

  const overview = overviewQuery.data
  const stats = statsQuery.data

  const marketDataDown = detectMarketDataOutage(overviewQuery, pairsQuery)

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-lg font-bold uppercase tracking-wider font-heading" style={{ color: 'var(--ink)' }}>
          Charts & Analytics
        </h1>
      </div>

      {marketDataDown && (
        <MarketDataServiceOutageBanner
          testId="charts-market-data-outage-banner"
          title={MARKET_DATA_SERVICE_OUTAGE_TITLE}
          lead={CHARTS_MARKET_DATA_OUTAGE_LEAD}
          onRetry={() => {
            void overviewQuery.refetch()
            void pairsQuery.refetch()
          }}
        />
      )}

      {(!marketDataDown || overviewQuery.isLoading || overview) && (
        <div className="shell-panel grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3">
          <StatBox
            label="24h Volume"
            value={overview ? formatNum(overview.total_volume_24h) : '—'}
            loading={overviewQuery.isLoading}
          />
          <StatBox
            label="24h Volume (USD)"
            value={
              overview?.total_volume_24h_usd != null && overview.total_volume_24h_usd !== ''
                ? formatNum(overview.total_volume_24h_usd, 2)
                : '—'
            }
            loading={overviewQuery.isLoading}
          />
          <StatBox
            label="USTC / USD"
            value={
              overview?.ustc_price_usd != null && overview.ustc_price_usd !== ''
                ? `$${formatNum(overview.ustc_price_usd, 6)}`
                : '—'
            }
            loading={overviewQuery.isLoading}
          />
          <StatBox
            label="24h Trades"
            value={overview ? overview.total_trades_24h.toLocaleString() : '—'}
            loading={overviewQuery.isLoading}
          />
          <StatBox
            label="Pairs"
            value={overview ? overview.pair_count.toString() : '—'}
            loading={overviewQuery.isLoading}
          />
          <StatBox
            label="Tokens"
            value={overview ? overview.token_count.toString() : '—'}
            loading={overviewQuery.isLoading}
          />
        </div>
      )}

      {/* Pair Selector */}
      <div className="shell-panel">
        <label htmlFor="chart-pair-search" className="label-glass mb-1 block">
          Find pair
        </label>
        <input
          id="chart-pair-search"
          type="search"
          className="input-glass w-full mb-3"
          placeholder="Search by symbol, pair address, or token…"
          value={pairSearch}
          onChange={(e) => setPairSearch(e.target.value)}
          aria-label="Filter pairs by symbol or address"
        />
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-3">
          <div>
            <label htmlFor="chart-pair-sort" className="label-glass mb-1 block">
              Sort
            </label>
            <MenuSelect
              id="chart-pair-sort"
              className="relative w-full"
              value={pairSort}
              options={CHARTS_PAIR_SORT_OPTIONS}
              onChange={(v) => {
                sounds.playButtonPress()
                const next = v as IndexerPairSort
                setPairSort(next)
                setPairPage(0)
                if (next === 'volume_24h') setPairOrder('desc')
              }}
            />
          </div>
          <div>
            <label htmlFor="chart-pair-order" className="label-glass mb-1 block">
              Order
            </label>
            <MenuSelect
              id="chart-pair-order"
              className="relative w-full"
              value={pairOrder}
              options={ORDER_OPTIONS}
              onChange={(v) => {
                sounds.playButtonPress()
                setPairOrder(v as 'asc' | 'desc')
                setPairPage(0)
              }}
            />
          </div>
        </div>
        <label htmlFor="chart-pair-select" className="label-glass mb-1 block">
          Select Pair
        </label>
        <MenuSelect
          id="chart-pair-select"
          className="relative w-full"
          aria-label="Select pair"
          value={activePairAddr}
          options={pairMenuOptions}
          disabled={pairOptions.length === 0}
          emptyLabel="No pairs yet"
          onChange={(v) => {
            sounds.playButtonPress()
            setSelectedPairAddr(v)
          }}
        />
        {pairTotal > PAIR_PAGE_SIZE && !pairsQuery.isLoading && !pairsQuery.isError && (
          <div className="flex items-center justify-between mt-3 pt-3 border-t border-white/10">
            <span className="text-xs uppercase tracking-wide" style={{ color: 'var(--ink-dim)' }}>
              Page {pairPage + 1} of {pairTotalPages} · {pairTotal.toLocaleString()} pair(s)
            </span>
            <div className="flex gap-2">
              <button
                type="button"
                className="btn-muted !text-xs"
                disabled={!canPairPrev}
                onClick={() => {
                  sounds.playButtonPress()
                  setPairPage((p) => Math.max(0, p - 1))
                }}
              >
                Previous
              </button>
              <button
                type="button"
                className="btn-muted !text-xs"
                disabled={!canPairNext}
                onClick={() => {
                  sounds.playButtonPress()
                  setPairPage((p) => p + 1)
                }}
              >
                Next
              </button>
            </div>
          </div>
        )}
        {needsPairFetch && selectedPairQuery.isLoading && (
          <p className="text-xs mt-2" style={{ color: 'var(--ink-subtle)' }}>
            Loading selected pair…
          </p>
        )}
        {pairsQuery.isSuccess && (pairItems?.length ?? 0) === 0 && !pairsQuery.isLoading && !marketDataDown && (
          <p className="text-xs mt-2 leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
            No pairs yet.
          </p>
        )}
      </div>

      {/* Price Chart */}
      {activePairAddr && (
        <div className="h-[min(70vh,720px)]">
          <PriceChart pairAddress={activePairAddr} tapeLastPriceUsd={tradesQuery.data?.[0]?.price} />
        </div>
      )}

      {/* 24h Stats */}
      {stats && activePair && (
        <div className="shell-panel">
          <h3
            className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading"
            style={{ color: 'var(--ink)' }}
          >
            24h Stats — {indexerPairMenuLabel(activePair, { variant: 'compact' })}
          </h3>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
            <StatBox label={`Vol (${activePair.asset_0.symbol})`} value={formatNum(stats.volume_base)} />
            <StatBox label={`Vol (${activePair.asset_1.symbol})`} value={formatNum(stats.volume_quote)} />
            <StatBox label="Trades" value={stats.trade_count.toLocaleString()} />
            <StatBox
              label="Price Change"
              value={
                stats.price_change_pct != null
                  ? `${stats.price_change_pct >= 0 ? '+' : ''}${stats.price_change_pct.toFixed(2)}%`
                  : '—'
              }
              color={
                stats.price_change_pct != null
                  ? stats.price_change_pct >= 0
                    ? 'var(--color-positive)'
                    : 'var(--color-negative)'
                  : undefined
              }
            />
            <StatBox label="High" value={stats.high ? formatNum(stats.high, 6) : '—'} />
            <StatBox label="Low" value={stats.low ? formatNum(stats.low, 6) : '—'} />
            <StatBox label="Open" value={stats.open_price ? formatNum(stats.open_price, 6) : '—'} />
            <StatBox label="Close" value={stats.close_price ? formatNum(stats.close_price, 6) : '—'} />
          </div>
        </div>
      )}

      {!statsQuery.isLoading && !stats && activePairAddr && (
        <div className="shell-panel text-center py-6" style={{ color: 'var(--ink-dim)' }}>
          <p className="text-sm">No trades yet.</p>
        </div>
      )}

      {/* TWAP Oracle Prices */}
      {activePairAddr && activePair && (
        <div className="shell-panel">
          <h3
            className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading"
            style={{ color: 'var(--ink)' }}
          >
            TWAP Oracle — {indexerPairMenuLabel(activePair, { variant: 'compact' })}
          </h3>
          <div className="grid grid-cols-3 gap-3">
            {TWAP_WINDOWS.map((w) => {
              const entry = twapQuery.data?.find((e) => e.label === w.label)
              return (
                <StatBox
                  key={w.label}
                  label={`TWAP ${w.label}`}
                  value={entry?.price != null ? formatNum(entry.price, 6) : '—'}
                  loading={twapQuery.isLoading}
                />
              )
            })}
          </div>
          {oracleInfoQuery.data && (
            <div className="mt-3 grid grid-cols-2 md:grid-cols-4 gap-3">
              <StatBox
                label="Obs count"
                value={`${oracleInfoQuery.data.observations_stored} / ${oracleInfoQuery.data.observation_cardinality}`}
              />
              <StatBox
                label="Oldest"
                value={
                  oracleInfoQuery.data.oldest_observation_timestamp > 0
                    ? formatTimeFromUnixSeconds(oracleInfoQuery.data.oldest_observation_timestamp)
                    : '—'
                }
              />
              <StatBox
                label="Newest"
                value={
                  oracleInfoQuery.data.newest_observation_timestamp > 0
                    ? formatTimeFromUnixSeconds(oracleInfoQuery.data.newest_observation_timestamp)
                    : '—'
                }
              />
              <StatBox label="Buffer size" value={oracleInfoQuery.data.observation_cardinality.toString()} />
            </div>
          )}
          {twapQuery.isError && (
            <p className="text-xs mt-2" style={{ color: 'var(--ink-subtle)' }}>
              Oracle data unavailable for this pair
            </p>
          )}
          {!twapQuery.isLoading && !twapQuery.isError && twapQuery.data?.every((e) => e.price === null) && (
            <p className="text-xs mt-2" style={{ color: 'var(--ink-subtle)' }}>
              TWAP building…
            </p>
          )}
        </div>
      )}

      {/* Recent Trades */}
      <div className="shell-panel-strong">
        <h3 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
          Recent Trades
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
          <TradesTable
            trades={tradesQuery.data}
            formatTimeFn={formatTime}
            activePair={activePair}
            ariaLabel="Recent trades"
          />
        )}
      </div>

      {/* Leaderboard */}
      <div className="shell-panel-strong">
        <h3 className="text-sm font-semibold uppercase tracking-wide mb-3 font-heading" style={{ color: 'var(--ink)' }}>
          Leaderboard
        </h3>

        <div className="flex gap-1 mb-4 flex-wrap" role="tablist" aria-label="Leaderboard sort">
          {LEADERBOARD_TABS.map((tab) => (
            <button
              key={tab.key}
              role="tab"
              aria-selected={leaderboardSort === tab.key}
              onClick={() => {
                sounds.playButtonPress()
                setLeaderboardSort(tab.key)
              }}
              className={`tab-glass !text-[10px] !px-3 !py-1.5 ${
                leaderboardSort === tab.key ? 'tab-glass-active' : 'tab-glass-inactive'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        {leaderboardQuery.isLoading && (
          <div className="space-y-2 py-4" aria-live="polite">
            {Array.from({ length: 5 }).map((_, i) => (
              <Skeleton key={i} height="1.5rem" />
            ))}
          </div>
        )}
        {leaderboardQuery.isError && (
          <RetryError message="Failed to load leaderboard" onRetry={() => void leaderboardQuery.refetch()} />
        )}
        {leaderboardQuery.data && leaderboardQuery.data.length === 0 && (
          <p className="text-center py-8 text-sm" style={{ color: 'var(--ink-dim)' }}>
            No traders yet
          </p>
        )}
        {leaderboardQuery.data && leaderboardQuery.data.length > 0 && (
          <div className="overflow-x-auto">
            <table className="w-full text-xs" aria-label="Trader leaderboard">
              <thead>
                <tr className="border-b border-white/10" style={{ color: 'var(--ink-dim)' }}>
                  <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">
                    #
                  </th>
                  <th scope="col" className="text-left py-2 px-2 font-medium uppercase tracking-wider">
                    Trader
                  </th>
                  <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                    {LEADERBOARD_TABS.find((t) => t.key === leaderboardSort)?.label ?? 'Value'}
                  </th>
                  <th scope="col" className="text-right py-2 px-2 font-medium uppercase tracking-wider">
                    Trades
                  </th>
                </tr>
              </thead>
              <tbody>
                {leaderboardQuery.data.map((trader: IndexerTrader, i: number) => {
                  const metricValue = getLeaderboardMetric(trader, leaderboardSort)
                  const isPnl = leaderboardSort !== 'total_volume'
                  return (
                    <tr key={trader.address} className="border-b border-white/5 hover:bg-white/5 transition-colors">
                      <td className="py-1.5 px-2 font-semibold" style={{ color: 'var(--ink-subtle)' }}>
                        {i + 1}
                      </td>
                      <td className="py-1.5 px-2">
                        <Link
                          to={`/trader/${trader.address}`}
                          className="hover:underline"
                          style={{ color: 'var(--mint)' }}
                          onClick={() => sounds.playButtonPress()}
                        >
                          {shortenAddress(trader.address, 10, 6)}
                        </Link>
                      </td>
                      <td
                        className="py-1.5 px-2 text-right font-medium"
                        style={{ color: isPnl ? undefined : 'var(--ink)' }}
                      >
                        {isPnl ? (
                          <PnlValue value={getLeaderboardPnlValue(trader, leaderboardSort)} />
                        ) : (
                          formatNum(metricValue)
                        )}
                      </td>
                      <td className="py-1.5 px-2 text-right" style={{ color: 'var(--ink-subtle)' }}>
                        {trader.total_trades.toLocaleString()}
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  )
}

function getLeaderboardMetric(trader: IndexerTrader, sort: string): string {
  switch (sort) {
    case 'best_trade_pnl':
      return trader.best_trade_pnl ?? ''
    case 'total_realized_pnl':
      return trader.total_realized_pnl
    case 'worst_trade_pnl':
      return trader.worst_trade_pnl ?? ''
    default:
      return trader.total_volume
  }
}

function getLeaderboardPnlValue(trader: IndexerTrader, sort: string): string | null {
  switch (sort) {
    case 'best_trade_pnl':
      return trader.best_trade_pnl
    case 'total_realized_pnl':
      return trader.total_realized_pnl
    case 'worst_trade_pnl':
      return trader.worst_trade_pnl
    default:
      return null
  }
}
