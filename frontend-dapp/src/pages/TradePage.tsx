import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useLocation, useNavigate, useParams } from 'react-router-dom'
import { useWalletStore } from '@/hooks/useWallet'
import { WalletIndexerHistoryPanel } from '@/components/trade/WalletIndexerHistoryPanel'
import { useIsFetching, useQuery, useQueryClient } from '@tanstack/react-query'
import { Panel, PanelGroup, PanelResizeHandle, type ImperativePanelHandle } from 'react-resizable-panels'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { getPair, getTrades } from '@/services/indexer/client'
import { getPairPaused } from '@/services/terraclassic/pair'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { PairSearchSelect, RetryError, Skeleton } from '@/components/ui'
import { LcdQueryGate } from '@/components/common/LcdQueryGate'
import { MarketDataServiceOutageBanner } from '@/components/common/MarketDataServiceOutageBanner'
import PriceChart from '@/components/charts/PriceChart'
import { OrderBookPanel } from '@/components/trade/OrderBookPanel'
import { TradeOrderTicket } from '@/components/trade/TradeOrderTicket'
import { InvalidPairLinkNotice } from '@/components/trade/InvalidPairLinkNotice'
import { PairNotFoundLinkNotice } from '@/components/trade/PairNotFoundLinkNotice'
import { TradePairSwitchStatus } from '@/components/trade/TradePairSwitchStatus'
import { TradePageWorkspaceSkeleton } from '@/components/trade/TradePageWorkspaceSkeleton'
import { useLimitOrderCancelMutation } from '@/hooks/useLimitOrderCancelMutation'
import { useQueryManualRetry } from '@/hooks/useQueryManualRetry'
import { sounds } from '@/lib/sounds'
import { pairInfoMenuLabel } from '@/utils/pairMenuOptions'
import { formatTime } from '@/utils/formatDate'
import { isIndexerPairNotFoundError } from '@/utils/indexerErrors'
import {
  TRADE_INDEXER_OUTAGE_BANNER_LEAD,
  TRADE_INDEXER_OUTAGE_BANNER_TAIL,
  TRADE_INDEXER_OUTAGE_BANNER_TITLE,
} from '@/utils/indexerTradeOutageCopy'
import { getErrorMessage } from '@/utils/humanizeUserFacingError'
import {
  getInvalidTradePairRouteParam,
  getTradePageInvalidLinkNotice,
  getTradePageUnknownPairNotice,
  getUnknownTradePairRouteParam,
  isKnownFactoryTradePair,
  isPendingTradePairRouteResolution,
  isTradePairRouteParam,
  shouldAutoPickDefaultTradePair,
  shouldShowTradeWorkspace,
} from '@/utils/tradePairRoute'
import { prefetchTradePairWorkspace } from '@/utils/tradePairPrefetch'
import { isTradePairWorkspaceQuery } from '@/utils/tradePairWorkspaceFetching'
import type { IndexerPair } from '@/types'
import type { LimitBookTicketDraft } from '@/types/limitBookTicketDraft'
import { useMediaQuery } from '@/hooks/useMediaQuery'
import { TradeRecentTradesSection } from '@/components/trade/TradeRecentTradesSection'
import { detectTradeIndexerOutage } from '@/utils/tradeIndexerOutage'
import { TRADE_DESKTOP_LAYOUT_MEDIA_QUERY } from '@/utils/tradePageLayout'
import { TradeOnboardingStrip } from '@/components/common/TradeOnboardingStrip'
import { TradeWorkspaceDisclosure } from '@/components/trade/TradeWorkspaceDisclosure'
import {
  TRADE_DESKTOP_TAPE_COLLAPSED_SIZE,
  TRADE_DESKTOP_TAPE_EXPANDED_SIZE,
  TRADE_TAPE_EXPANDED_KEY,
  TRADE_WALLET_HISTORY_EXPANDED_KEY,
  readTradePanelExpanded,
  writeTradePanelExpanded,
} from '@/utils/tradeWorkspacePanels'

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
  return <div className="card-glass !p-2 flex-1 min-h-0 flex flex-col">{chart}</div>
}

export default function TradePage() {
  const { pairAddr: routePair } = useParams<{ pairAddr?: string }>()
  const location = useLocation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const invalidRoutePair = useMemo(() => getInvalidTradePairRouteParam(routePair), [routePair])
  const invalidLinkNotice = getTradePageInvalidLinkNotice(location.state)
  const unknownPairNotice = getTradePageUnknownPairNotice(location.state)
  const [pairAddr, setPairAddr] = useState('')

  const clearLinkNotices = useCallback(() => {
    navigate('/trade', { replace: true, state: null })
  }, [navigate])

  const pairsQuery = useQuery({
    queryKey: ['allPairs'],
    queryFn: () => getAllPairsPaginated(),
    staleTime: 60_000,
  })

  const pairs = useMemo(() => pairsQuery.data?.pairs ?? [], [pairsQuery.data])
  const factoryPairsResolved = pairsQuery.isSuccess
  const unknownRoutePair = useMemo(
    () => (invalidRoutePair ? null : getUnknownTradePairRouteParam(routePair, pairs, factoryPairsResolved)),
    [invalidRoutePair, routePair, pairs, factoryPairsResolved]
  )
  const pendingDeepLinkPair = useMemo(
    () => isPendingTradePairRouteResolution(routePair, invalidRoutePair, factoryPairsResolved),
    [routePair, invalidRoutePair, factoryPairsResolved]
  )
  const activePairMenuLabel = useMemo(() => {
    const hit = pairs.find((p) => p.contract_addr === pairAddr)
    return hit ? pairInfoMenuLabel(hit, { variant: 'full' }) : undefined
  }, [pairs, pairAddr])
  const pairRouteReady = isTradePairRouteParam(pairAddr)
  const showTradeWorkspace = shouldShowTradeWorkspace({
    pairRouteReady,
    invalidLinkNotice,
    unknownPairNotice,
    pendingDeepLinkPair,
  })

  useEffect(() => {
    if (invalidRoutePair) {
      setPairAddr('')
      if (routePair) {
        navigate('/trade', { replace: true, state: { invalidPair: invalidRoutePair } })
      }
      return
    }
    if (unknownRoutePair) {
      setPairAddr('')
      if (routePair) {
        navigate('/trade', { replace: true, state: { unknownPair: unknownRoutePair } })
      }
      return
    }
    if (isKnownFactoryTradePair(routePair, pairs)) {
      setPairAddr(routePair)
      if (invalidLinkNotice || unknownPairNotice) {
        navigate(`/trade/${routePair}`, { replace: true, state: null })
      }
    }
  }, [invalidRoutePair, unknownRoutePair, routePair, pairs, navigate, invalidLinkNotice, unknownPairNotice])

  useEffect(() => {
    if (
      pairAddr ||
      pairs.length === 0 ||
      invalidLinkNotice ||
      unknownPairNotice ||
      !shouldAutoPickDefaultTradePair({
        routePair,
        invalidRoutePair,
        unknownRoutePair,
        pendingDeepLinkPair,
      })
    ) {
      return
    }
    const first = pairs[0]?.contract_addr
    if (first) {
      setPairAddr(first)
      navigate(`/trade/${first}`, { replace: true })
    }
  }, [
    pairAddr,
    pairs,
    navigate,
    routePair,
    invalidRoutePair,
    unknownRoutePair,
    pendingDeepLinkPair,
    invalidLinkNotice,
    unknownPairNotice,
  ])

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
  const indexerDown = detectTradeIndexerOutage(indexerPairQuery, tradesQuery)

  useEffect(() => {
    if (!factoryPairsResolved || !routePair || invalidRoutePair) return
    if (isKnownFactoryTradePair(routePair, pairs)) return
    if (!indexerPairQuery.isError || !isIndexerPairNotFoundError(indexerPairQuery.error)) return
    setPairAddr('')
    navigate('/trade', { replace: true, state: { unknownPair: routePair } })
  }, [
    factoryPairsResolved,
    routePair,
    invalidRoutePair,
    pairs,
    indexerPairQuery.isError,
    indexerPairQuery.error,
    navigate,
  ])
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
  const showWorkspaceSkeleton = pairsQuery.isLoading
  const tapePanelRef = useRef<ImperativePanelHandle>(null)
  const [desktopTapeExpanded, setDesktopTapeExpanded] = useState(() =>
    readTradePanelExpanded(TRADE_TAPE_EXPANDED_KEY, false)
  )

  const expandDesktopTape = useCallback(() => {
    tapePanelRef.current?.expand?.()
    setDesktopTapeExpanded(true)
    writeTradePanelExpanded(TRADE_TAPE_EXPANDED_KEY, true)
  }, [])

  const collapseDesktopTape = useCallback(() => {
    tapePanelRef.current?.collapse?.()
    setDesktopTapeExpanded(false)
    writeTradePanelExpanded(TRADE_TAPE_EXPANDED_KEY, false)
  }, [])

  // react-resizable-panels may fire onExpand during initial layout; keep first visit collapsed (GitLab #417).
  useEffect(() => {
    if (readTradePanelExpanded(TRADE_TAPE_EXPANDED_KEY, false)) return
    let cancelled = false
    const id = requestAnimationFrame(() => {
      if (cancelled) return
      try {
        tapePanelRef.current?.collapse?.()
      } catch {
        // PanelGroup may not be registered yet in unit tests (GitLab #417).
      }
      setDesktopTapeExpanded(false)
    })
    return () => {
      cancelled = true
      cancelAnimationFrame(id)
    }
  }, [])

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
      if (isTradePairRouteParam(addr)) {
        prefetchTradePairWorkspace(queryClient, addr)
        setPairAddr(addr)
        navigate(`/trade/${addr}`, { state: null })
      }
    },
    [navigate, queryClient]
  )

  return (
    <div className="space-y-3">
      <div>
        <h1
          className="text-lg font-bold uppercase tracking-wider font-heading"
          style={{ color: 'var(--ink)' }}
          data-testid="trade-page-heading"
        >
          Trade
        </h1>
        <p className="text-sm mt-1 max-w-3xl" style={{ color: 'var(--ink-dim)' }} data-testid="trade-page-subtitle">
          Order book, chart, tape, and limit plus market tickets. Indexer book reads proxy LCD (paginated depth).
        </p>
      </div>

      <TradeOnboardingStrip />

      {invalidLinkNotice && (
        <InvalidPairLinkNotice
          invalidParam={invalidLinkNotice}
          pairSelectId={TRADE_PAIR_SELECT_ID}
          onDismiss={clearLinkNotices}
        />
      )}

      {unknownPairNotice && (
        <PairNotFoundLinkNotice
          unknownParam={unknownPairNotice}
          pairSelectId={TRADE_PAIR_SELECT_ID}
          onDismiss={clearLinkNotices}
        />
      )}

      {indexerDown && (
        <MarketDataServiceOutageBanner
          layout="inline"
          testId="trade-indexer-outage-banner"
          title={TRADE_INDEXER_OUTAGE_BANNER_TITLE}
          lead={TRADE_INDEXER_OUTAGE_BANNER_LEAD}
          tail={TRADE_INDEXER_OUTAGE_BANNER_TAIL}
        />
      )}

      <div className="shell-panel p-3">
        <label className="label-glass mb-1 block" htmlFor={TRADE_PAIR_SELECT_ID}>
          Pair
        </label>
        <LcdQueryGate query={pairsQuery} loadingFallback={<Skeleton height="2.5rem" width="100%" />}>
          <PairSearchSelect
            id={TRADE_PAIR_SELECT_ID}
            className="relative w-full max-w-xl shrink-0"
            aria-label="Trading pair"
            value={pairAddr}
            factoryPairs={pairs}
            emptyLabel="No pairs on factory"
            onChange={onPairChange}
            onOptionIntent={onPairPrefetchIntent}
          />
        </LcdQueryGate>
      </div>

      {showWorkspaceSkeleton ? <TradePageWorkspaceSkeleton /> : null}
      {!showWorkspaceSkeleton && showTradeWorkspace && showPairSwitchLoading && (
        <TradePairSwitchStatus pairLabel={activePairMenuLabel} />
      )}

      {/*
        Sub-desktop layout: single column <768px; tablet 768–1023px uses a 2-col top row
        (chart | order ticket) with order book + tape below — see docs/frontend.md § Trade page
        responsive layout (GitLab #146). Only one TradeOrderTicket mounts at a time (GitLab #178).
      */}
      {!showWorkspaceSkeleton && showTradeWorkspace && !isTradeDesktopLayout && (
        <div className="lg:hidden grid grid-cols-1 gap-3 md:grid-cols-2" data-testid="trade-sub-lg-workspace">
          <div className="min-h-[280px] md:col-span-2 md:row-start-2">
            <OrderBookPanel pairAddress={pairAddr} pair={activePair} {...orderBookPanelProps} />
          </div>
          <div className="min-h-0 md:col-start-2 md:row-start-1 flex flex-col" data-testid="trade-sub-lg-ticket-col">
            {tradeOrderTicket}
          </div>
          <div
            className="min-h-[220px] md:min-h-[280px] md:col-start-1 md:row-start-1 flex flex-col"
            data-testid="trade-sub-lg-chart-col"
          >
            <TradeChartSlot {...chartSlotProps} />
          </div>
          <div className="card-glass !p-3 md:col-span-2 md:row-start-3">
            <TradeWorkspaceDisclosure
              title="Recent trades"
              storageKey={TRADE_TAPE_EXPANDED_KEY}
              defaultExpanded={false}
              testId="trade-sub-lg-tape-disclosure"
            >
              <TradeRecentTradesSection
                pairRouteReady={pairRouteReady}
                tradesQuery={tradesQuery}
                activePair={activePair}
                formatTimeFn={formatTime}
                skeletonHeight="6rem"
                hideHeading
              />
            </TradeWorkspaceDisclosure>
          </div>
        </div>
      )}

      {!showWorkspaceSkeleton && showTradeWorkspace && isTradeDesktopLayout && (
        <div className="hidden lg:block h-[min(85vh,920px)] min-h-[440px]" data-testid="trade-desktop-workspace">
          <PanelGroup direction="horizontal" className="h-full gap-0">
            <Panel defaultSize={24} minSize={18} className="min-w-0">
              <OrderBookPanel pairAddress={pairAddr} pair={activePair} {...orderBookPanelProps} />
            </Panel>
            <TradeResizeHandleVertical />
            <Panel defaultSize={52} minSize={35} className="min-w-0 flex flex-col">
              <PanelGroup direction="vertical" className="h-full flex-1 min-h-0">
                <Panel defaultSize={58} minSize={30} className="min-h-0">
                  <div className="h-full min-h-[200px] card-glass !p-2 overflow-hidden flex flex-col min-h-0">
                    <TradeChartSlot {...chartSlotProps} wrapInCard={false} />
                  </div>
                </Panel>
                <TradeResizeHandleHorizontal />
                <Panel
                  ref={tapePanelRef}
                  defaultSize={
                    desktopTapeExpanded ? TRADE_DESKTOP_TAPE_EXPANDED_SIZE : TRADE_DESKTOP_TAPE_COLLAPSED_SIZE
                  }
                  minSize={TRADE_DESKTOP_TAPE_COLLAPSED_SIZE}
                  collapsedSize={TRADE_DESKTOP_TAPE_COLLAPSED_SIZE}
                  collapsible
                  className="min-h-0"
                  onExpand={() => setDesktopTapeExpanded(true)}
                  onCollapse={() => setDesktopTapeExpanded(false)}
                >
                  <div className="h-full flex flex-col min-h-0 card-glass !p-3" data-testid="trade-desktop-tape-panel">
                    <div className="flex items-center justify-between gap-2 mb-2 shrink-0">
                      <h2 className="text-xs font-semibold uppercase tracking-wide" style={{ color: 'var(--ink-dim)' }}>
                        Recent trades
                      </h2>
                      <button
                        type="button"
                        className="btn-muted !text-[10px] !px-2 !py-1"
                        data-testid="trade-desktop-tape-toggle"
                        onClick={() => (desktopTapeExpanded ? collapseDesktopTape() : expandDesktopTape())}
                      >
                        {desktopTapeExpanded ? 'Collapse' : 'Expand'}
                      </button>
                    </div>
                    {desktopTapeExpanded ? (
                      <div className="flex-1 min-h-0 overflow-y-auto">
                        <TradeRecentTradesSection
                          pairRouteReady={pairRouteReady}
                          tradesQuery={tradesQuery}
                          activePair={activePair}
                          formatTimeFn={formatTime}
                          skeletonHeight="5rem"
                          hideHeading
                        />
                      </div>
                    ) : (
                      <p className="text-[10px] leading-snug" style={{ color: 'var(--ink-subtle)' }}>
                        Collapsed by default — expand to view the live tape, or drag the resize handle above.
                      </p>
                    )}
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

      {address && showTradeWorkspace && (
        <TradeWorkspaceDisclosure
          title="Wallet swap history"
          storageKey={TRADE_WALLET_HISTORY_EXPANDED_KEY}
          defaultExpanded={false}
          testId="trade-wallet-history-disclosure"
          className="mt-3"
        >
          <WalletIndexerHistoryPanel walletAddress={address} pairAddress={pairAddr} sections={['swaps']} />
        </TradeWorkspaceDisclosure>
      )}
    </div>
  )
}
