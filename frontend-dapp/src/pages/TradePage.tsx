import { useEffect, useMemo, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { Panel, PanelGroup, PanelResizeHandle } from 'react-resizable-panels'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { getPair, getTrades, INDEXER_URL } from '@/services/indexer/client'
import { MenuSelect, TradesTable, RetryError, Skeleton } from '@/components/ui'
import PriceChart from '@/components/charts/PriceChart'
import { OrderBookPanel } from '@/components/trade/OrderBookPanel'
import { TradeOrderTicket } from '@/components/trade/TradeOrderTicket'
import { sounds } from '@/lib/sounds'
import { pairInfosToMenuSelectOptions } from '@/utils/pairMenuOptions'
import { formatTime } from '@/utils/formatDate'
import { isIndexerUnavailableError } from '@/utils/indexerErrors'
import type { IndexerPair } from '@/types'

function TradeResizeHandleVertical() {
  return <PanelResizeHandle className="w-1.5 rounded-md bg-white/10 hover:bg-white/20 transition-colors shrink-0" />
}

function TradeResizeHandleHorizontal() {
  return <PanelResizeHandle className="h-1.5 rounded-md bg-white/10 hover:bg-white/20 transition-colors shrink-0" />
}

export default function TradePage() {
  const { pairAddr: routePair } = useParams<{ pairAddr?: string }>()
  const navigate = useNavigate()
  const [pairAddr, setPairAddr] = useState(routePair ?? '')

  const pairsQuery = useQuery({
    queryKey: ['allPairs'],
    queryFn: () => getAllPairsPaginated(),
    staleTime: 60_000,
  })

  const pairs = useMemo(() => pairsQuery.data?.pairs ?? [], [pairsQuery.data])
  const pairMenuOptions = useMemo(() => pairInfosToMenuSelectOptions(pairs, { variant: 'full' }), [pairs])

  useEffect(() => {
    if (routePair && routePair.startsWith('terra1')) {
      setPairAddr(routePair)
    }
  }, [routePair])

  useEffect(() => {
    if (pairAddr || pairs.length === 0) return
    const first = pairs[0]?.contract_addr
    if (first) {
      setPairAddr(first)
      navigate(`/trade/${first}`, { replace: true })
    }
  }, [pairAddr, pairs, navigate])

  const indexerPairQuery = useQuery({
    queryKey: ['indexer-pair-trade', pairAddr],
    queryFn: () => getPair(pairAddr),
    enabled: pairAddr.startsWith('terra1'),
    staleTime: 60_000,
    retry: false,
  })

  const tradesQuery = useQuery({
    queryKey: ['pair-trades-trade', pairAddr],
    queryFn: () => getTrades(pairAddr, 80),
    enabled: pairAddr.startsWith('terra1'),
    refetchInterval: 15_000,
    retry: false,
  })

  const activePair: IndexerPair | undefined = indexerPairQuery.data
  const indexerDown = indexerPairQuery.isError && isIndexerUnavailableError(indexerPairQuery.error)

  const onPairChange = (addr: string) => {
    sounds.playButtonPress()
    setPairAddr(addr)
    if (addr.startsWith('terra1')) {
      navigate(`/trade/${addr}`)
    }
  }

  return (
    <div className="space-y-3">
      <div>
        <h1 className="text-lg font-bold uppercase tracking-wider font-heading" style={{ color: 'var(--ink)' }}>
          Trade
        </h1>
        <p className="text-sm mt-1" style={{ color: 'var(--ink-dim)' }}>
          Order book, chart, tape, and limit orders — indexer book reads proxy LCD (paginated depth).
        </p>
      </div>

      {indexerDown && (
        <div className="alert-warning text-sm" role="alert">
          Indexer unavailable at <code className="font-mono text-[11px]">{INDEXER_URL}</code> — chart and tape may be
          limited. Order book and tickets still use chain where applicable.
        </div>
      )}

      <div className="shell-panel p-3">
        <label className="label-neo mb-1 block" htmlFor="trade-pair-select">
          Pair
        </label>
        {pairsQuery.isLoading && <Skeleton height="2.5rem" width="100%" />}
        {!pairsQuery.isLoading && (
          <MenuSelect
            id="trade-pair-select"
            className="relative w-full max-w-xl"
            aria-label="Trading pair"
            value={pairAddr}
            options={pairMenuOptions}
            emptyLabel="No pairs on factory"
            onChange={onPairChange}
          />
        )}
      </div>

      {/* Mobile / small: stacked */}
      <div className="lg:hidden space-y-3">
        <div className="min-h-[280px]">
          <OrderBookPanel pairAddress={pairAddr} />
        </div>
        <TradeOrderTicket pairAddr={pairAddr} pairs={pairs} pairsLoading={pairsQuery.isLoading} />
        {indexerPairQuery.isLoading && <Skeleton height="12rem" />}
        {indexerPairQuery.isError && !indexerDown && (
          <RetryError message={(indexerPairQuery.error as Error).message} onRetry={() => indexerPairQuery.refetch()} />
        )}
        {activePair && (
          <div className="card-neo !p-2">
            <PriceChart pairAddress={pairAddr} />
          </div>
        )}
        <div className="card-neo !p-3">
          <h2 className="text-xs font-semibold uppercase tracking-wide mb-2" style={{ color: 'var(--ink-dim)' }}>
            Recent trades
          </h2>
          {tradesQuery.isLoading && <Skeleton height="6rem" />}
          {tradesQuery.data && (
            <TradesTable
              trades={tradesQuery.data}
              formatTimeFn={formatTime}
              activePair={activePair}
              ariaLabel="Recent trades"
            />
          )}
        </div>
      </div>

      {/* Desktop: resizable panels */}
      <div className="hidden lg:block h-[min(85vh,920px)] min-h-[440px]">
        <PanelGroup direction="horizontal" className="h-full gap-0">
          <Panel defaultSize={24} minSize={18} className="min-w-0">
            <OrderBookPanel pairAddress={pairAddr} />
          </Panel>
          <TradeResizeHandleVertical />
          <Panel defaultSize={52} minSize={35} className="min-w-0 flex flex-col">
            <PanelGroup direction="vertical" className="h-full flex-1 min-h-0">
              <Panel defaultSize={58} minSize={30} className="min-h-0">
                <div className="h-full min-h-[200px] card-neo !p-2 overflow-hidden">
                  {indexerPairQuery.isLoading && <Skeleton height="100%" />}
                  {indexerPairQuery.isError && !indexerDown && (
                    <RetryError
                      message={(indexerPairQuery.error as Error).message}
                      onRetry={() => indexerPairQuery.refetch()}
                    />
                  )}
                  {activePair && <PriceChart pairAddress={pairAddr} />}
                  {!pairAddr.startsWith('terra1') && (
                    <p className="text-sm p-4" style={{ color: 'var(--ink-dim)' }}>
                      Select a pair for the chart.
                    </p>
                  )}
                </div>
              </Panel>
              <TradeResizeHandleHorizontal />
              <Panel defaultSize={42} minSize={22} className="min-h-0">
                <div className="h-full flex flex-col min-h-0 card-neo !p-3">
                  <h2
                    className="text-xs font-semibold uppercase tracking-wide mb-2 shrink-0"
                    style={{ color: 'var(--ink-dim)' }}
                  >
                    Recent trades
                  </h2>
                  <div className="flex-1 min-h-0 overflow-y-auto">
                    {tradesQuery.isLoading && <Skeleton height="5rem" />}
                    {tradesQuery.data && (
                      <TradesTable
                        trades={tradesQuery.data}
                        formatTimeFn={formatTime}
                        activePair={activePair}
                        ariaLabel="Recent trades"
                      />
                    )}
                  </div>
                </div>
              </Panel>
            </PanelGroup>
          </Panel>
          <TradeResizeHandleVertical />
          <Panel defaultSize={24} minSize={18} className="min-w-0 min-h-0">
            <TradeOrderTicket pairAddr={pairAddr} pairs={pairs} pairsLoading={pairsQuery.isLoading} />
          </Panel>
        </PanelGroup>
      </div>
    </div>
  )
}
