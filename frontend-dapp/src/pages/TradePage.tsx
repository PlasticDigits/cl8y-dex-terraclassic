import { useCallback, useEffect, useMemo, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { useWalletStore } from '@/hooks/useWallet'
import { WalletIndexerHistoryPanel } from '@/components/trade/WalletIndexerHistoryPanel'
import { useIsFetching, useQuery, useQueryClient } from '@tanstack/react-query'
import { Panel, PanelGroup, PanelResizeHandle } from 'react-resizable-panels'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { getPair, getTrades } from '@/services/indexer/client'
import { getPairPaused } from '@/services/terraclassic/pair'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { MenuSelect, TradesTable, RetryError, Skeleton } from '@/components/ui'
import { LcdQueryGate } from '@/components/common/LcdQueryGate'
import PriceChart from '@/components/charts/PriceChart'
import { OrderBookPanel } from '@/components/trade/OrderBookPanel'
import { TradeOrderTicket } from '@/components/trade/TradeOrderTicket'
import { InvalidPairLinkNotice } from '@/components/trade/InvalidPairLinkNotice'
import { TradePairSwitchStatus } from '@/components/trade/TradePairSwitchStatus'
import { useLimitOrderCancelMutation } from '@/hooks/useLimitOrderCancelMutation'
import { useQueryManualRetry } from '@/hooks/useQueryManualRetry'
import { sounds } from '@/lib/sounds'
import { pairInfosToMenuSelectOptions } from '@/utils/pairMenuOptions'
import { formatTime } from '@/utils/formatDate'
import { isIndexerUnavailableError } from '@/utils/indexerErrors'
import {
  TRADE_INDEXER_OUTAGE_BANNER_LEAD,
  TRADE_INDEXER_OUTAGE_BANNER_TAIL,
  TRADE_INDEXER_OUTAGE_BANNER_TITLE,
} from '@/utils/indexerTradeOutageCopy'
import { getErrorMessage } from '@/utils/humanizeUserFacingError'
import { getInvalidTradePairRouteParam, isTradePairRouteParam } from '@/utils/tradePairRoute'
import { prefetchTradePairWorkspace } from '@/utils/tradePairPrefetch'
import { isTradePairWorkspaceQuery } from '@/utils/tradePairWorkspaceFetching'
import type { IndexerPair } from '@/types'
import type { LimitBookTicketDraft } from '@/types/limitBookTicketDraft'
import { useMediaQuery } from '@/hooks/useMediaQuery'
import { TRADE_DESKTOP_LAYOUT_MEDIA_QUERY } from '@/utils/tradePageLayout'

const TRADE_PAIR_SELECT_ID = 'trade-pair-select'

function TradeResizeHandleVertical() {
  return <PanelResizeHandle className="w-1.5 rounded-md bg-white/10 hover:bg-white/20 transition-colors shrink-0" />
}

function TradeResizeHandleHorizontal() {
  return <PanelResizeHandle className="h-1.5 rounded-md bg-white/10 hover:bg-white/20 transition-colors shrink-0" />
}

type TradeChartSlotProps = {
  pairRouteReady: boolean
  pairAddr: string
  showRetryError: boolean
  retryMessage: string
  isRetrying: boolean
  onRetry: () => void
  tapeLastPriceUsd?: string | null
  /** When false, chart fills the parent card without an extra wrapper (desktop panel). */
  wrapInCard?: boolean
}

/** Chart mounts on `pairAddr` immediately; candles load in parallel with `getPair` (GitLab #180). */
function TradeChartSlot({
  pairRouteReady,
  pairAddr,
  showRetryError,
  retryMessage,
  isRetrying,
  onRetry,
  tapeLastPriceUsd,
  wrapInCard = true,
}: TradeChartSlotProps) {
  if (!pairRouteReady) {
    return (
      <p className="text-sm p-4" style={{ color: 'var(--ink-dim)' }}>
        Select a pair for the chart.
      </p>
    )
  }
  if (showRetryError) {
    return (
      <RetryError
        data-testid="trade-chart-retry-error"
        message={retryMessage}
        isRetrying={isRetrying}
        onRetry={onRetry}
      />
    )
  }
  const chart = <PriceChart pairAddress={pairAddr} tapeLastPriceUsd={tapeLastPriceUsd} />
  if (!wrapInCard) return chart
  return <div className="card-neo !p-2 flex-1 min-h-0 flex flex-col">{chart}</div>
}

export default function TradePage() {
  const { pairAddr: routePair } = useParams<{ pairAddr?: string }>()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const invalidRoutePair = useMemo(() => getInvalidTradePairRouteParam(routePair), [routePair])
  const [invalidLinkNotice, setInvalidLinkNotice] = useState<string | null>(null)
  const [pairAddr, setPairAddr] = useState(() => (isTradePairRouteParam(routePair) ? routePair : ''))

  const pairsQuery = useQuery({
    queryKey: ['allPairs'],
    queryFn: () => getAllPairsPaginated(),
    staleTime: 60_000,
  })

  const pairs = useMemo(() => pairsQuery.data?.pairs ?? [], [pairsQuery.data])
  const pairMenuOptions = useMemo(() => pairInfosToMenuSelectOptions(pairs, { variant: 'full' }), [pairs])
  const pairRouteReady = isTradePairRouteParam(pairAddr)
  const activePairMenuLabel = useMemo(
    () => pairMenuOptions.find((o) => o.value === pairAddr)?.label,
    [pairMenuOptions, pairAddr]
  )

  useEffect(() => {
    if (invalidRoutePair) {
      setInvalidLinkNotice(invalidRoutePair)
      setPairAddr('')
      if (routePair) {
        navigate('/trade', { replace: true })
      }
      return
    }
    if (isTradePairRouteParam(routePair)) {
      setPairAddr(routePair)
      setInvalidLinkNotice(null)
    }
  }, [invalidRoutePair, routePair, navigate])

  useEffect(() => {
    if (pairAddr || pairs.length === 0 || invalidLinkNotice) return
    const first = pairs[0]?.contract_addr
    if (first) {
      setPairAddr(first)
      navigate(`/trade/${first}`, { replace: true })
    }
  }, [pairAddr, pairs, navigate, invalidLinkNotice])

  useEffect(() => {
    if (!pairRouteReady) return
    prefetchTradePairWorkspace(queryClient, pairAddr)
  }, [pairRouteReady, pairAddr, queryClient])

  const indexerPairQuery = useQuery({
    queryKey: ['indexer-pair-trade', pairAddr],
    queryFn: () => getPair(pairAddr),
    enabled: pairRouteReady,
    staleTime: 60_000,
    retry: false,
  })

  const tradesQuery = useQuery({
    queryKey: ['pair-trades-trade', pairAddr],
    queryFn: () => getTrades(pairAddr, 80),
    enabled: pairRouteReady,
    refetchInterval: 15_000,
    retry: false,
  })

  const activePair: IndexerPair | undefined = indexerPairQuery.data
  const indexerDown = indexerPairQuery.isError && isIndexerUnavailableError(indexerPairQuery.error)
  const indexerPairRetry = useQueryManualRetry(['indexer-pair-trade', pairAddr], indexerPairQuery)
  const showIndexerPairRetryError = indexerPairQuery.isError && !indexerDown && !indexerPairRetry.isRetrying

  const tradeWorkspaceFetchingCount = useIsFetching({
    predicate: (query) => pairRouteReady && isTradePairWorkspaceQuery(query, pairAddr),
  })
  const showPairSwitchLoading = pairRouteReady && tradeWorkspaceFetchingCount > 0

  const address = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)
  const wallet = getConnectedWallet()
  const isWalletConnected = !!address && !!wallet

  const pausedQuery = useQuery({
    queryKey: ['pairPaused', pairAddr],
    queryFn: () => getPairPaused(pairAddr),
    enabled: pairRouteReady,
    staleTime: 15_000,
  })
  const isPairPaused = pausedQuery.data?.paused === true

  const limitCancelMutation = useLimitOrderCancelMutation(pairAddr, address ?? undefined)

  const factoryPair = useMemo(() => pairs.find((p) => p.contract_addr === pairAddr), [pairs, pairAddr])

  const [limitBookDraftKey, setLimitBookDraftKey] = useState(0)
  const [limitBookDraft, setLimitBookDraft] = useState<LimitBookTicketDraft | null>(null)

  const pushLimitBookDraft = useCallback((draft: LimitBookTicketDraft) => {
    setLimitBookDraft(draft)
    setLimitBookDraftKey((k) => k + 1)
  }, [])

  const onLimitBookDraftConsumed = useCallback(() => setLimitBookDraft(null), [])

  const isTradeDesktopLayout = useMediaQuery(TRADE_DESKTOP_LAYOUT_MEDIA_QUERY)

  const chartSlotProps = {
    pairRouteReady,
    pairAddr,
    showRetryError: showIndexerPairRetryError,
    retryMessage: getErrorMessage(indexerPairQuery.error),
    isRetrying: indexerPairRetry.isRetrying,
    onRetry: indexerPairRetry.retry,
    tapeLastPriceUsd: tradesQuery.data?.[0]?.price,
  }

  const tradeOrderTicket = (
    <TradeOrderTicket
      pairAddr={pairAddr}
      pairs={pairs}
      pairsLoading={pairsQuery.isLoading}
      indexerPair={activePair}
      latestTrade={tradesQuery.data?.[0]}
      tapeHeadlineUsd={tradesQuery.data?.[0]?.price}
      cancelLimitOrderMutation={limitCancelMutation}
      limitBookDraftKey={limitBookDraftKey}
      limitBookDraft={limitBookDraft}
      onLimitBookDraftConsumed={onLimitBookDraftConsumed}
    />
  )

  const orderBookPanelProps = {
    walletAddress: address ?? undefined,
    isWalletConnected,
    isPairPaused,
    openWalletModal,
    cancelLimitOrderMutation: limitCancelMutation,
    onPrefillLimitTicket: pushLimitBookDraft,
    factoryPair,
  }

  const onPairPrefetchIntent = useCallback(
    (addr: string) => {
      if (!isTradePairRouteParam(addr) || addr === pairAddr) return
      prefetchTradePairWorkspace(queryClient, addr)
    },
    [pairAddr, queryClient]
  )

  const onPairChange = useCallback(
    (addr: string) => {
      sounds.playButtonPress()
      setInvalidLinkNotice(null)
      if (isTradePairRouteParam(addr)) {
        prefetchTradePairWorkspace(queryClient, addr)
        setPairAddr(addr)
        navigate(`/trade/${addr}`)
      }
    },
    [navigate, queryClient]
  )

  return (
    <div className="space-y-3">
      <div>
        <h1 className="text-lg font-bold uppercase tracking-wider font-heading" style={{ color: 'var(--ink)' }}>
          Trade
        </h1>
        <p className="text-sm mt-1" style={{ color: 'var(--ink-dim)' }}>
          Order book, chart, tape, and limit plus market tickets — indexer book reads proxy LCD (paginated depth).
        </p>
      </div>

      {invalidLinkNotice && (
        <InvalidPairLinkNotice
          invalidParam={invalidLinkNotice}
          pairSelectId={TRADE_PAIR_SELECT_ID}
          onDismiss={() => setInvalidLinkNotice(null)}
        />
      )}

      {indexerDown && (
        <div className="alert-warning text-sm" role="alert" data-testid="trade-indexer-outage-banner">
          <span className="font-semibold">{TRADE_INDEXER_OUTAGE_BANNER_TITLE}</span> {TRADE_INDEXER_OUTAGE_BANNER_LEAD}{' '}
          {TRADE_INDEXER_OUTAGE_BANNER_TAIL}
        </div>
      )}

      <div className="shell-panel p-3">
        <label className="label-neo mb-1 block" htmlFor={TRADE_PAIR_SELECT_ID}>
          Pair
        </label>
        <LcdQueryGate query={pairsQuery} loadingFallback={<Skeleton height="2.5rem" width="100%" />}>
          <MenuSelect
            id={TRADE_PAIR_SELECT_ID}
            className="relative w-full max-w-xl"
            aria-label="Trading pair"
            value={pairAddr}
            options={pairMenuOptions}
            emptyLabel="No pairs on factory"
            onChange={onPairChange}
            onOptionIntent={onPairPrefetchIntent}
          />
        </LcdQueryGate>
      </div>

      {showPairSwitchLoading && <TradePairSwitchStatus pairLabel={activePairMenuLabel} />}

      {/*
        Sub-desktop layout: single column <768px; tablet 768–1023px uses a 2-col top row
        (chart | order ticket) with order book + tape below — see docs/frontend.md § Trade page
        responsive layout (GitLab #146). Only one TradeOrderTicket mounts at a time (GitLab #178).
      */}
      {!isTradeDesktopLayout && (
        <div className="lg:hidden grid grid-cols-1 gap-3 md:grid-cols-2" data-testid="trade-sub-lg-workspace">
          <div className="min-h-[280px] md:col-span-2 md:row-start-2">
            <OrderBookPanel pairAddress={pairAddr} pair={activePair} {...orderBookPanelProps} />
          </div>
          <div className="min-h-0 md:col-start-2 md:row-start-1 flex flex-col">{tradeOrderTicket}</div>
          <div className="min-h-[220px] md:min-h-[280px] md:col-start-1 md:row-start-1 flex flex-col">
            <TradeChartSlot {...chartSlotProps} />
          </div>
          <div className="card-neo !p-3 md:col-span-2 md:row-start-3">
            <h2 className="text-xs font-semibold uppercase tracking-wide mb-2" style={{ color: 'var(--ink-dim)' }}>
              Recent trades
            </h2>
            {pairRouteReady && tradesQuery.isLoading && <Skeleton height="6rem" />}
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
      )}

      {isTradeDesktopLayout && (
        <div className="hidden lg:block h-[min(85vh,920px)] min-h-[440px]" data-testid="trade-desktop-workspace">
          <PanelGroup direction="horizontal" className="h-full gap-0">
            <Panel defaultSize={24} minSize={18} className="min-w-0">
              <OrderBookPanel pairAddress={pairAddr} pair={activePair} {...orderBookPanelProps} />
            </Panel>
            <TradeResizeHandleVertical />
            <Panel defaultSize={52} minSize={35} className="min-w-0 flex flex-col">
              <PanelGroup direction="vertical" className="h-full flex-1 min-h-0">
                <Panel defaultSize={58} minSize={30} className="min-h-0">
                  <div className="h-full min-h-[200px] card-neo !p-2 overflow-hidden flex flex-col min-h-0">
                    <TradeChartSlot {...chartSlotProps} wrapInCard={false} />
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
                      {pairRouteReady && tradesQuery.isLoading && <Skeleton height="5rem" />}
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
              {tradeOrderTicket}
            </Panel>
          </PanelGroup>
        </div>
      )}

      {address && pairRouteReady && (
        <div className="mt-3">
          <WalletIndexerHistoryPanel walletAddress={address} pairAddress={pairAddr} sections={['swaps']} />
        </div>
      )}
    </div>
  )
}
