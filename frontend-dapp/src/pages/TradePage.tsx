import { useCallback, useEffect, useMemo, useState } from 'react'
import { useLocation, useNavigate, useParams } from 'react-router-dom'
import { useWalletStore } from '@/hooks/useWallet'
import { WalletIndexerHistoryPanel } from '@/components/trade/WalletIndexerHistoryPanel'
import { useIsFetching, useQuery, useQueryClient } from '@tanstack/react-query'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { getOraclePrice, getPair, getTrades } from '@/services/indexer/client'
import { getPairPaused } from '@/services/terraclassic/pair'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { PairSearchSelect, PairTokenLinks, RetryError, Skeleton } from '@/components/ui'
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
import {
  firstCatalogPairAddress,
  isRetailHiddenTestPair,
  pairInfoLegIds,
  pairInfoLegSymbols,
  retailExposeTestTokens,
} from '@/utils/pairCatalogRank'
import { getTokenDisplaySymbol } from '@/utils/tokenDisplay'
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
import { assetInfoLabel, type IndexerPair } from '@/types'
import type { LimitBookTicketDraft } from '@/types/limitBookTicketDraft'
import { useMediaQuery } from '@/hooks/useMediaQuery'
import { TradeRecentTradesSection } from '@/components/trade/TradeRecentTradesSection'
import { detectTradeIndexerOutage } from '@/utils/tradeIndexerOutage'
import { resolveDisplayTapeLastPriceUsd } from '@/utils/pairPriceUsd'
import { usePairDisplayOrientation } from '@/hooks/usePairDisplayOrientation'
import { TRADE_DESKTOP_LAYOUT_MEDIA_QUERY } from '@/utils/tradePageLayout'
import { TradeOnboardingStrip } from '@/components/common/TradeOnboardingStrip'
import { TradeWorkspaceDisclosure } from '@/components/trade/TradeWorkspaceDisclosure'
import { TradeDesktopWorkspace } from '@/components/trade/TradeDesktopWorkspace'
import {
  TRADE_BOOK_VISIBLE_KEY,
  TRADE_TAPE_EXPANDED_KEY,
  TRADE_TICKET_VISIBLE_KEY,
  TRADE_WALLET_HISTORY_EXPANDED_KEY,
  readTradePanelExpanded,
  readTradePanelVisible,
  writeTradePanelExpanded,
  writeTradePanelVisible,
} from '@/utils/tradeWorkspacePanels'

const TRADE_PAIR_SELECT_ID = 'trade-pair-select'

type TradeChartSlotProps = {
  pairRouteReady: boolean
  pairAddr: string
  showRetryError: boolean
  retryMessage: string
  isRetrying: boolean
  onRetry: () => void
  tapeLastPriceUsd?: string | null
  displayInverted?: boolean
  onToggleDisplayInvert?: () => void
  pairPillLabel?: string
  invertAriaLabel?: string
  displayBaseSymbol?: string
  volumeBaseDecimals?: number
  volumeQuoteDecimals?: number
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
  displayInverted,
  onToggleDisplayInvert,
  pairPillLabel,
  invertAriaLabel,
  displayBaseSymbol,
  volumeBaseDecimals,
  volumeQuoteDecimals,
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
  const chart = (
    <PriceChart
      pairAddress={pairAddr}
      tapeLastPriceUsd={tapeLastPriceUsd}
      displayInverted={displayInverted}
      onToggleDisplayInvert={onToggleDisplayInvert}
      pairPillLabel={pairPillLabel}
      invertAriaLabel={invertAriaLabel}
      displayBaseSymbol={displayBaseSymbol}
      volumeBaseDecimals={volumeBaseDecimals}
      volumeQuoteDecimals={volumeQuoteDecimals}
    />
  )
  return <div className="flex-1 min-h-0 h-full flex flex-col">{chart}</div>
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
    const first = firstCatalogPairAddress(pairs)
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

  const ustcOracleQuery = useQuery({
    queryKey: ['indexer-oracle-price', 'ustc'],
    queryFn: () => getOraclePrice('ustc'),
    staleTime: 60_000,
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
  const showLegacyGemNotice = useMemo(() => {
    if (!factoryPair || retailExposeTestTokens()) return false
    const [id0, id1] = pairInfoLegIds(factoryPair)
    const [s0, s1] = pairInfoLegSymbols(factoryPair)
    return isRetailHiddenTestPair(s0, s1, id0, id1)
  }, [factoryPair])

  const [limitBookDraftKey, setLimitBookDraftKey] = useState(0)
  const [limitBookDraft, setLimitBookDraft] = useState<LimitBookTicketDraft | null>(null)
  const [bookVisible, setBookVisible] = useState(() => readTradePanelVisible(TRADE_BOOK_VISIBLE_KEY, true))
  const [ticketVisible, setTicketVisible] = useState(() => readTradePanelVisible(TRADE_TICKET_VISIBLE_KEY, true))
  const [desktopTapeExpanded, setDesktopTapeExpanded] = useState(() =>
    readTradePanelExpanded(TRADE_TAPE_EXPANDED_KEY, false)
  )

  const persistBookVisible = useCallback((visible: boolean) => {
    setBookVisible(visible)
    writeTradePanelVisible(TRADE_BOOK_VISIBLE_KEY, visible)
  }, [])

  const persistTicketVisible = useCallback((visible: boolean) => {
    setTicketVisible(visible)
    writeTradePanelVisible(TRADE_TICKET_VISIBLE_KEY, visible)
  }, [])

  const persistTapeExpanded = useCallback((expanded: boolean) => {
    setDesktopTapeExpanded(expanded)
    writeTradePanelExpanded(TRADE_TAPE_EXPANDED_KEY, expanded)
  }, [])

  const pushLimitBookDraft = useCallback(
    (draft: LimitBookTicketDraft) => {
      persistTicketVisible(true)
      setLimitBookDraft(draft)
      setLimitBookDraftKey((k) => k + 1)
    },
    [persistTicketVisible]
  )

  const onLimitBookDraftConsumed = useCallback(() => setLimitBookDraft(null), [])

  const isTradeDesktopLayout = useMediaQuery(TRADE_DESKTOP_LAYOUT_MEDIA_QUERY)
  const showWorkspaceSkeleton = pairsQuery.isLoading

  const factoryToken0 = factoryPair ? assetInfoLabel(factoryPair.asset_infos[0]) : ''
  const factoryToken1 = factoryPair ? assetInfoLabel(factoryPair.asset_infos[1]) : ''
  const token0Symbol = activePair?.asset_0.symbol ?? (factoryToken0 ? getTokenDisplaySymbol(factoryToken0) : 'Base')
  const token1Symbol = activePair?.asset_1.symbol ?? (factoryToken1 ? getTokenDisplaySymbol(factoryToken1) : 'Quote')
  const pairOrientation = usePairDisplayOrientation({
    pairAddr,
    asset0: activePair?.asset_0 ?? (factoryToken0 ? { contractAddr: factoryToken0, symbol: token0Symbol } : null),
    asset1: activePair?.asset_1 ?? (factoryToken1 ? { contractAddr: factoryToken1, symbol: token1Symbol } : null),
    token0Symbol,
    token1Symbol,
  })

  const factoryTapeLastPriceUsd = useMemo(
    () =>
      resolveDisplayTapeLastPriceUsd({
        inverted: false,
        priceUsd: tradesQuery.data?.[0]?.price_usd,
        price: tradesQuery.data?.[0]?.price,
        decimalsBase: activePair?.asset_0.decimals,
        decimalsQuote: activePair?.asset_1.decimals,
        quoteSymbol: activePair?.asset_1.symbol,
        quoteDenom: activePair?.asset_1.denom,
        ustcUsd: ustcOracleQuery.data?.price_usd,
      }),
    [tradesQuery.data, activePair, ustcOracleQuery.data?.price_usd]
  )

  const tapeLastPriceUsd = useMemo(
    () =>
      resolveDisplayTapeLastPriceUsd({
        inverted: pairOrientation.inverted,
        priceUsd: tradesQuery.data?.[0]?.price_usd,
        price: tradesQuery.data?.[0]?.price,
        decimalsBase: activePair?.asset_0.decimals,
        decimalsQuote: activePair?.asset_1.decimals,
        quoteSymbol: activePair?.asset_1.symbol,
        quoteDenom: activePair?.asset_1.denom,
        ustcUsd: ustcOracleQuery.data?.price_usd,
        displayBaseSymbol: pairOrientation.displayBase,
        displayBaseDenom: pairOrientation.inverted ? activePair?.asset_1.denom : activePair?.asset_0.denom,
      }),
    [
      tradesQuery.data,
      activePair,
      ustcOracleQuery.data?.price_usd,
      pairOrientation.inverted,
      pairOrientation.displayBase,
    ]
  )

  const chartSlotProps = {
    pairRouteReady,
    pairAddr,
    showRetryError: showIndexerPairRetryError,
    retryMessage: getErrorMessage(indexerPairQuery.error),
    isRetrying: indexerPairRetry.isRetrying,
    onRetry: indexerPairRetry.retry,
    tapeLastPriceUsd,
    displayInverted: pairOrientation.inverted,
    onToggleDisplayInvert: pairOrientation.toggleInverted,
    pairPillLabel: pairOrientation.pillLabel,
    invertAriaLabel: pairOrientation.invertAriaLabel,
    displayBaseSymbol: pairOrientation.displayBase,
    volumeBaseDecimals: activePair?.asset_0.decimals,
    volumeQuoteDecimals: activePair?.asset_1.decimals,
  }

  const tradeOrderTicket = (
    <TradeOrderTicket
      pairAddr={pairAddr}
      pairs={pairs}
      pairsLoading={pairsQuery.isLoading}
      indexerPair={activePair}
      latestTrade={tradesQuery.data?.[0]}
      tapeHeadlineUsd={tapeLastPriceUsd}
      factoryTapeHeadlineUsd={factoryTapeLastPriceUsd}
      displayInverted={pairOrientation.inverted}
      onToggleDisplayInvert={pairOrientation.toggleInverted}
      invertAriaLabel={pairOrientation.invertAriaLabel}
      cancelLimitOrderMutation={limitCancelMutation}
      limitBookDraftKey={limitBookDraftKey}
      limitBookDraft={limitBookDraft}
      onLimitBookDraftConsumed={onLimitBookDraftConsumed}
      interactive={!isTradeDesktopLayout || ticketVisible}
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
    displayInverted: pairOrientation.inverted,
    displayBaseSymbol: pairOrientation.displayBase,
    displayQuoteSymbol: pairOrientation.displayQuote,
  }

  const displayPairMenuLabel = useMemo(() => {
    const hit = pairs.find((p) => p.contract_addr === pairAddr)
    return hit ? pairInfoMenuLabel(hit, { variant: 'full', displayInverted: pairOrientation.inverted }) : undefined
  }, [pairs, pairAddr, pairOrientation.inverted])

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

      <div className="shell-panel p-3" data-testid="trade-pair-select-panel">
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
            selectedLabelOverride={displayPairMenuLabel}
          />
        </LcdQueryGate>
        {showTradeWorkspace && factoryPair ? (
          <PairTokenLinks
            pairAddress={factoryPair.contract_addr}
            asset0={factoryPair.asset_infos[0]}
            asset1={factoryPair.asset_infos[1]}
            inverted={pairOrientation.inverted}
            liquidityUsd={
              indexerPairQuery.data?.pair_address === factoryPair.contract_addr
                ? indexerPairQuery.data.liquidity_usd
                : undefined
            }
          />
        ) : null}
        {showTradeWorkspace && showLegacyGemNotice ? (
          <p className="mt-2 text-xs" style={{ color: 'var(--ink-dim)' }} data-testid="trade-legacy-gem-notice">
            Legacy noneconomic market.
          </p>
        ) : null}
      </div>

      {showWorkspaceSkeleton ? <TradePageWorkspaceSkeleton /> : null}
      {!showWorkspaceSkeleton && showTradeWorkspace && showPairSwitchLoading && (
        <TradePairSwitchStatus pairLabel={displayPairMenuLabel ?? activePairMenuLabel} />
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
          <div
            className="min-h-0 md:col-start-2 md:row-start-1 flex flex-col md:max-h-[min(72vh,44rem)]"
            data-testid="trade-sub-lg-ticket-col"
          >
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
                inverted={pairOrientation.inverted}
              />
            </TradeWorkspaceDisclosure>
          </div>
        </div>
      )}

      {!showWorkspaceSkeleton && showTradeWorkspace && isTradeDesktopLayout && (
        <TradeDesktopWorkspace
          bookVisible={bookVisible}
          ticketVisible={ticketVisible}
          tapeExpanded={desktopTapeExpanded}
          onBookVisibleChange={persistBookVisible}
          onTicketVisibleChange={persistTicketVisible}
          onTapeExpandedChange={persistTapeExpanded}
          book={<OrderBookPanel pairAddress={pairAddr} pair={activePair} {...orderBookPanelProps} />}
          chart={<TradeChartSlot {...chartSlotProps} />}
          ticket={tradeOrderTicket}
          tape={
            <TradeRecentTradesSection
              pairRouteReady={pairRouteReady}
              tradesQuery={tradesQuery}
              activePair={activePair}
              formatTimeFn={formatTime}
              skeletonHeight="5rem"
              hideHeading
              inverted={pairOrientation.inverted}
            />
          }
        />
      )}

      {address && showTradeWorkspace && (
        <TradeWorkspaceDisclosure
          title="Wallet swap history"
          storageKey={TRADE_WALLET_HISTORY_EXPANDED_KEY}
          defaultExpanded={false}
          testId="trade-wallet-history-disclosure"
          className="mt-3"
        >
          <WalletIndexerHistoryPanel
            walletAddress={address}
            pairAddress={pairAddr}
            sections={['swaps']}
            activePair={activePair}
            inverted={pairOrientation.inverted}
          />
        </TradeWorkspaceDisclosure>
      )}
    </div>
  )
}
