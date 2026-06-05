import { useMemo, useState, useEffect, useId, useCallback } from 'react'
import { useQuery, useQueryClient, useIsFetching } from '@tanstack/react-query'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { useWalletStore } from '@/hooks/useWallet'
import { usePairLimitCancellations } from '@/hooks/usePairLimitCancellations'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { placeLimitOrderWithAllowance, getPairPaused } from '@/services/terraclassic/pair'
import { estimateLimitOrderPlaceSequenceUlunaFeesTotal } from '@/services/terraclassic/transactions'
import { getPairLimitPlacements, getPair, getTrades } from '@/services/indexer/client'
import { sounds } from '@/lib/sounds'
import { MenuSelect, TxResultAlert, Spinner } from '@/components/ui'
import { TerraBroadcastPendingLink } from '@/components/ui/TerraBroadcastPendingLink'
import { terraBroadcastPendingButtonLabel } from '@/utils/terraBroadcastUi'
import { LcdQueryGate } from '@/components/common/LcdQueryGate'
import { MarketDataServiceOutageBanner } from '@/components/common/MarketDataServiceOutageBanner'
import { MarketDataLoadingStatus } from '@/components/common/MarketDataLoadingStatus'
import { detectMarketDataOutage } from '@/utils/marketDataOutage'
import { LIMITS_MARKET_DATA_OUTAGE_LEAD, MARKET_DATA_SERVICE_OUTAGE_TITLE } from '@/utils/marketDataServiceCopy'
import { TRADE_INDEXER_OUTAGE_BANNER_TAIL } from '@/utils/indexerTradeOutageCopy'
import { assetInfoLabel, tokenAssetInfo, type IndexerPair } from '@/types'
import { formatNum, getDecimals, toRawAmount } from '@/utils/formatAmount'
import { evaluateLimitOrderEscrowPlaceGate } from '@/utils/limitOrderEscrowBalanceGate'
import { evaluateLimitOrderNativeGasPlaceGate } from '@/utils/limitOrderNativeGasBalanceGate'
import { warnIndexerPlacementPollFailed } from '@/utils/warnIndexerPlacementPollFailed'
import { pairInfosToMenuSelectOptions } from '@/utils/pairMenuOptions'
import { fetchCW20TokenInfo, getTokenDisplaySymbol, shortenAddress } from '@/utils/tokenDisplay'
import { orderIdHasIndexedCancellation } from '@/utils/limitOrderCancelUserMessage'
import { DOCS_GITLAB_BASE } from '@/utils/constants'
import { useLimitOrderPriceRefBundle } from '@/hooks/useLimitOrderPriceRefBundle'
import { useLimitOrderForm } from '@/hooks/useLimitOrderForm'
import { useLimitEscrowMaxReapply } from '@/hooks/useLimitEscrowMaxReapply'
import { useLimitOrderEscrowBalance } from '@/hooks/useLimitOrderEscrowBalance'
import { useNativeUlunaBalance } from '@/hooks/useNativeUlunaBalance'
import { LimitOrderAdvancedLimitSettings } from '@/components/trade/LimitOrderAdvancedLimitSettings'
import { LimitOrderBidAskSideSelector } from '@/components/trade/LimitOrderBidAskSideSelector'
import { LimitOrderEscrowAmountField } from '@/components/trade/LimitOrderEscrowAmountField'
import { LimitOrderEscrowPlaceGuardMessage } from '@/components/trade/LimitOrderEscrowPlaceGuardMessage'
import { LimitOrderExpiryField } from '@/components/trade/LimitOrderExpiryField'
import { LimitOrderMyPlacementsPanel } from '@/components/trade/LimitOrderMyPlacementsPanel'
import { LimitOrderPreSubmitSummary } from '@/components/trade/LimitOrderPreSubmitSummary'
import { evaluateLimitOrderPricePlaceGate } from '@/utils/limitOrderPricePlaceGate'
import { escrowAmountUsdAnchorNotional, parsePositivePriceHuman } from '@/utils/limitOrderPriceReference'
import { LimitOrderLadderPanel } from '@/components/trade/LimitOrderLadderPanel'
import { LimitOrderPlaceLimitHeading, LimitOrderPriceInputWithContext } from '@/components/trade/LimitOrderPriceField'
import { WalletIndexerHistoryPanel } from '@/components/trade/WalletIndexerHistoryPanel'
import { OrderBookPanel } from '@/components/trade/OrderBookPanel'
import { useLimitOrderCancelMutation } from '@/hooks/useLimitOrderCancelMutation'
import { useLimitBookInfinite } from '@/hooks/useLimitBookInfinite'
import { useLimitOrderMakerFeeRates } from '@/hooks/useLimitOrderMakerFeeRates'
import { flattenLimitBookPages, resolveLimitInsertHintAfter } from '@/utils/limitBookInsertHint'
import type { LimitBookTicketDraft } from '@/types/limitBookTicketDraft'

export default function LimitOrdersPage() {
  const address = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)
  const wallet = getConnectedWallet()
  const isWalletConnected = !!address && !!wallet
  const queryClient = useQueryClient()
  const limitOrdersPriceInputId = useId()
  const limitOrdersCancelOrderInputId = useId()

  const [pairAddr, setPairAddr] = useState('')
  const [side, setSide] = useState<'bid' | 'ask'>('bid')
  const [price, setPrice] = useState('1')
  const {
    maxSteps,
    setMaxSteps,
    expiresAt,
    setExpiresAt,
    amountHuman,
    escrowAmountSource,
    onLimitAmountInputChange,
    onLimitAmountMax,
    resetLimitEscrowAmount,
    setLimitEscrowAmountFromDraft,
    setLimitEscrowAmountFromMaxReapply,
    limitAdvancedOpen,
    setLimitAdvancedOpen,
  } = useLimitOrderForm()
  const [cancelOrderId, setCancelOrderId] = useState('')
  const [lastIndexedOrderId, setLastIndexedOrderId] = useState<number | null>(null)
  const [placeMode, setPlaceMode] = useState<'single' | 'ladder'>('single')

  const pairsQuery = useQuery({
    queryKey: ['allPairs'],
    queryFn: () => getAllPairsPaginated(),
    staleTime: 60_000,
  })

  const pairs = pairsQuery.data?.pairs ?? []

  const pairMenuOptions = useMemo(() => pairInfosToMenuSelectOptions(pairs, { variant: 'full' }), [pairs])

  const selectedPair = useMemo(() => pairs.find((p) => p.contract_addr === pairAddr), [pairs, pairAddr])

  const token0 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[0]) : ''
  const token1 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[1]) : ''
  const escrowToken = side === 'bid' ? token1 : token0

  const escrowDecimals = escrowToken ? getDecimals(tokenAssetInfo(escrowToken)) : 6
  const escrowBalanceQuery = useLimitOrderEscrowBalance(address, escrowToken)
  const nativeUlunaQuery = useNativeUlunaBalance(address)

  const limitPlaceMinUlunaFees = useMemo(() => estimateLimitOrderPlaceSequenceUlunaFeesTotal(), [])

  const limitBookQuery = useLimitBookInfinite(pairAddr, side)
  const placeInsertHintAfter = useMemo(() => {
    const { orders, hasMore } = flattenLimitBookPages(limitBookQuery.data?.pages)
    return resolveLimitInsertHintAfter(side, price, orders, { hasMore })
  }, [limitBookQuery.data?.pages, side, price])

  const {
    effectiveFeeBps,
    makerPlacementFeeBps,
    feeLoading: limitFeeLoading,
    feeError: limitFeeError,
  } = useLimitOrderMakerFeeRates(pairAddr, address ?? undefined)

  const placementsQuery = useQuery({
    queryKey: ['limitPlacements', pairAddr],
    queryFn: () => getPairLimitPlacements(pairAddr, { limit: 100 }),
    enabled: pairAddr.startsWith('terra1'),
  })

  const cancellationsQuery = usePairLimitCancellations(pairAddr)

  const pausedQuery = useQuery({
    queryKey: ['pairPaused', pairAddr],
    queryFn: () => getPairPaused(pairAddr),
    enabled: pairAddr.startsWith('terra1'),
    staleTime: 15_000,
  })

  const indexerPairQuery = useQuery({
    queryKey: ['indexer-pair-limit-orders', pairAddr],
    queryFn: () => getPair(pairAddr),
    enabled: pairAddr.startsWith('terra1'),
    staleTime: 60_000,
    retry: false,
  })

  const tradesForLimitQuery = useQuery({
    queryKey: ['pair-trades-limit-orders', pairAddr],
    queryFn: () => getTrades(pairAddr, 40),
    enabled: pairAddr.startsWith('terra1'),
    refetchInterval: 15_000,
    retry: false,
  })

  const indexerPair: IndexerPair | undefined = indexerPairQuery.data
  const latestTrade = tradesForLimitQuery.data?.[0]
  const tapeHeadlineUsd = latestTrade?.price

  const marketDataDown = detectMarketDataOutage(indexerPairQuery, tradesForLimitQuery)

  const limitsWorkspaceFetchingCount = useIsFetching({
    predicate: (query) =>
      pairAddr.startsWith('terra1') &&
      (query.queryKey[0] === 'indexer-pair-limit-orders' || query.queryKey[0] === 'pair-trades-limit-orders'),
  })
  const showPairSwitchLoading = pairAddr.startsWith('terra1') && limitsWorkspaceFetchingCount > 0

  const { refToken1PerToken0, refSource, refResolutionLoading, refResolutionError } = useLimitOrderPriceRefBundle({
    pairAddr,
    selectedPair,
    indexerPair,
    latestTrade,
  })

  const escrowUsdNotionalApprox = useMemo(() => {
    const amt = parsePositivePriceHuman(amountHuman)
    if (amt == null) return null
    const usd = escrowAmountUsdAnchorNotional(amt, side === 'ask', refToken1PerToken0, tapeHeadlineUsd)
    if (usd == null || !Number.isFinite(usd)) return null
    return `$${formatNum(usd, 4)}`
  }, [amountHuman, side, refToken1PerToken0, tapeHeadlineUsd])

  const handleSideChange = useCallback(
    (next: 'bid' | 'ask') => {
      if (next === side) return
      setSide(next)
      if (escrowAmountSource === 'max') {
        setLimitEscrowAmountFromMaxReapply('')
      } else {
        resetLimitEscrowAmount()
      }
    },
    [side, escrowAmountSource, setLimitEscrowAmountFromMaxReapply, resetLimitEscrowAmount]
  )

  useLimitEscrowMaxReapply({
    escrowAmountSource,
    balanceQuery: escrowBalanceQuery,
    escrowDecimals,
    assetIsNativeUluna: escrowToken === 'uluna',
    limitPlaceRungCount: 1,
    setLimitEscrowAmountFromMaxReapply,
  })

  const isPaused = pausedQuery.data?.paused === true

  const limitCancelMutation = useLimitOrderCancelMutation(pairAddr, address ?? undefined)

  const onPrefillLimitTicketFromBook = (draft: LimitBookTicketDraft) => {
    setSide(draft.side)
    setPrice(draft.price)
    setLimitEscrowAmountFromDraft(draft.amountHuman)
  }

  const placeEscrowGate = useMemo(
    () =>
      evaluateLimitOrderEscrowPlaceGate(amountHuman, escrowDecimals, {
        data: escrowBalanceQuery.data,
        isLoading: escrowBalanceQuery.isLoading,
        isError: escrowBalanceQuery.isError,
      }),
    [amountHuman, escrowDecimals, escrowBalanceQuery.data, escrowBalanceQuery.isLoading, escrowBalanceQuery.isError]
  )

  const placeNativeGasGate = useMemo(
    () =>
      evaluateLimitOrderNativeGasPlaceGate(
        amountHuman,
        escrowDecimals,
        {
          data: nativeUlunaQuery.data,
          isLoading: nativeUlunaQuery.isLoading,
          isError: nativeUlunaQuery.isError,
        },
        limitPlaceMinUlunaFees
      ),
    [
      amountHuman,
      escrowDecimals,
      nativeUlunaQuery.data,
      nativeUlunaQuery.isLoading,
      nativeUlunaQuery.isError,
      limitPlaceMinUlunaFees,
    ]
  )

  const placePriceGate = useMemo(
    () =>
      evaluateLimitOrderPricePlaceGate(side, price, refToken1PerToken0, {
        refResolutionLoading,
        refResolutionError,
      }),
    [side, price, refToken1PerToken0, refResolutionLoading, refResolutionError]
  )

  const expiryPastBlocker = useMemo(() => {
    if (expiresAt === null) return null
    const nowSec = Math.floor(Date.now() / 1000)
    if (expiresAt <= nowSec) return 'Expiry must be in the future.'
    return null
  }, [expiresAt])

  const placeLimitCombinedOk =
    placeEscrowGate.canPlaceLimit &&
    placeNativeGasGate.canPlaceLimit &&
    placePriceGate.canPlaceLimit &&
    !expiryPastBlocker

  const placeLimitInlineGate = expiryPastBlocker
    ? { canPlaceLimit: false, userMessage: expiryPastBlocker, tone: 'warning' as const }
    : !placePriceGate.canPlaceLimit
      ? placePriceGate
      : placeEscrowGate.userMessage
        ? placeEscrowGate
        : placeNativeGasGate

  const myPlacements = useMemo(() => {
    if (!address || !placementsQuery.data) return []
    return placementsQuery.data.filter((r) => r.owner === address)
  }, [address, placementsQuery.data])

  const parsedCancelOrderId = parseInt(cancelOrderId, 10)
  const cancelIdIndexedAsCancelled =
    Number.isFinite(parsedCancelOrderId) &&
    parsedCancelOrderId >= 1 &&
    orderIdHasIndexedCancellation(cancellationsQuery.data ?? [], parsedCancelOrderId)

  const placeMutation = useTerraBroadcastMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Connect wallet')
      if (!selectedPair) throw new Error('Select a pair')
      if (!escrowToken.startsWith('terra1')) throw new Error('Escrow token must be CW20')
      const escrowGate = evaluateLimitOrderEscrowPlaceGate(amountHuman, escrowDecimals, escrowBalanceQuery)
      if (!escrowGate.canPlaceLimit) {
        if (!escrowGate.userMessage) throw new Error('Enter amount')
        throw new Error(escrowGate.userMessage)
      }
      const nativeGate = evaluateLimitOrderNativeGasPlaceGate(
        amountHuman,
        escrowDecimals,
        nativeUlunaQuery,
        limitPlaceMinUlunaFees
      )
      if (!nativeGate.canPlaceLimit) {
        if (!nativeGate.userMessage) throw new Error('Insufficient LUNC for gas')
        throw new Error(nativeGate.userMessage)
      }
      const priceGate = evaluateLimitOrderPricePlaceGate(side, price, refToken1PerToken0, {
        refResolutionLoading,
        refResolutionError,
      })
      if (!priceGate.canPlaceLimit) {
        throw new Error(priceGate.userMessage ?? 'Invalid limit price for this side.')
      }
      const raw = toRawAmount(amountHuman, escrowDecimals)
      if (raw === '0') throw new Error('Enter amount')
      return placeLimitOrderWithAllowance(
        address,
        escrowToken,
        selectedPair.contract_addr,
        raw,
        side,
        price,
        maxSteps,
        expiresAt,
        placeInsertHintAfter
      )
    },
    onSuccess: async () => {
      sounds.playSuccess()
      resetLimitEscrowAmount()
      queryClient.invalidateQueries({ queryKey: ['limitPlacements'] })
      queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPage', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPagePreview', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['limitOrderPricePoolRef', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['wallet-indexer-history'] })
      setLastIndexedOrderId(null)
      const addr = pairAddr
      const wallet = address
      if (!addr.startsWith('terra1') || !wallet) return
      for (let i = 0; i < 24; i++) {
        await new Promise((r) => setTimeout(r, 500))
        try {
          const rows = await getPairLimitPlacements(addr, { limit: 100 })
          const mine = rows.filter((r) => r.owner === wallet)
          const maxId = mine.reduce((m, r) => Math.max(m, r.order_id), 0)
          if (maxId > 0) {
            setLastIndexedOrderId(maxId)
            setCancelOrderId(String(maxId))
            break
          }
        } catch (err) {
          warnIndexerPlacementPollFailed(err)
        }
      }
    },
    onError: () => sounds.playError(),
  })

  const submitCancelFromForm = () => {
    if (!isWalletConnected) {
      openWalletModal()
      return
    }
    const id = parseInt(cancelOrderId, 10)
    if (!Number.isFinite(id) || id < 1) return
    limitCancelMutation.mutate(id, {
      onSuccess: () => {
        setCancelOrderId('')
        setLastIndexedOrderId(null)
      },
    })
  }

  useEffect(() => {
    pairs.forEach((p) => {
      const a = assetInfoLabel(p.asset_infos[0])
      const b = assetInfoLabel(p.asset_infos[1])
      if (a.startsWith('terra1')) void fetchCW20TokenInfo(a)
      if (b.startsWith('terra1')) void fetchCW20TokenInfo(b)
    })
  }, [pairs])

  useEffect(() => {
    setLastIndexedOrderId(null)
  }, [pairAddr])

  return (
    <div className="max-w-[560px] mx-auto">
      <div className="relative">
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-x-6 top-8 h-[78%] rounded-[28px] theme-hero-glow blur-2xl"
        />
        <div className="shell-panel-strong relative z-10">
          <div className="mb-6">
            <h1 className="text-lg font-semibold uppercase tracking-wide font-heading">Limit Orders</h1>
            <p className="text-sm mt-1" style={{ color: 'var(--ink-dim)' }}>
              Place or cancel on-chain limits on a pair. Bids escrow token1; asks escrow token0 (pair ordering).
            </p>
          </div>

          <LcdQueryGate
            query={pairsQuery}
            loadingFallback={
              <div className="flex items-center gap-2 py-8 justify-center">
                <Spinner />
              </div>
            }
          >
            <div className="space-y-6">
              <div>
                <label className="label-neo" htmlFor="limit-pair">
                  Pair
                </label>
                <MenuSelect
                  id="limit-pair"
                  className="relative w-full"
                  aria-label="Trading pair"
                  value={pairAddr}
                  options={pairMenuOptions}
                  emptyLabel="No pairs on factory"
                  onChange={setPairAddr}
                />
              </div>

              {selectedPair && (
                <div className="text-xs uppercase tracking-wide font-medium" style={{ color: 'var(--ink-dim)' }}>
                  Token0: {shortenAddress(token0)} · Token1: {shortenAddress(token1)}
                </div>
              )}

              {selectedPair && marketDataDown && (
                <MarketDataServiceOutageBanner
                  testId="limits-market-data-outage-banner"
                  title={MARKET_DATA_SERVICE_OUTAGE_TITLE}
                  lead={LIMITS_MARKET_DATA_OUTAGE_LEAD}
                  tail={TRADE_INDEXER_OUTAGE_BANNER_TAIL}
                />
              )}

              {selectedPair && showPairSwitchLoading && (
                <MarketDataLoadingStatus testId="limits-pair-switch-loading" label="Loading market data…" />
              )}

              {selectedPair && isPaused && (
                <div className="alert-error text-sm space-y-2" role="status">
                  <p>
                    This pair is paused by governance. New limit orders, cancel, and parked-expiry Claim refund are
                    unavailable until the pair is unpaused (invariant L6 / GitLab #120 — all maker withdrawals frozen).
                    Escrow remains in the pair contract until unpause.
                  </p>
                  <p className="text-xs opacity-90">
                    <a
                      className="underline hover:opacity-80"
                      href={`${DOCS_GITLAB_BASE}/contracts-security-audit.md`}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      Security audit (L6)
                    </a>
                    {' · '}
                    <a
                      className="underline hover:opacity-80"
                      href={`${DOCS_GITLAB_BASE}/limit-orders.md`}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      limit-orders.md
                    </a>
                  </p>
                </div>
              )}

              {selectedPair && (
                <div className="card-neo !p-3 min-h-[22rem] flex flex-col">
                  <OrderBookPanel
                    pairAddress={pairAddr}
                    pair={indexerPair}
                    walletAddress={address ?? undefined}
                    isWalletConnected={isWalletConnected}
                    isPairPaused={isPaused}
                    openWalletModal={openWalletModal}
                    cancelLimitOrderMutation={limitCancelMutation}
                    onPrefillLimitTicket={onPrefillLimitTicketFromBook}
                    factoryPair={selectedPair}
                  />
                </div>
              )}

              <div className="card-neo !p-4 space-y-4">
                <div className="flex gap-2" role="tablist" aria-label="Place mode">
                  <button
                    type="button"
                    className={placeMode === 'single' ? 'btn-primary' : 'btn-neo'}
                    onClick={() => setPlaceMode('single')}
                  >
                    Single
                  </button>
                  <button
                    type="button"
                    className={placeMode === 'ladder' ? 'btn-primary' : 'btn-neo'}
                    onClick={() => setPlaceMode('ladder')}
                    data-testid="limit-place-mode-ladder"
                  >
                    Ladder
                  </button>
                </div>
                {placeMode === 'ladder' && selectedPair && address && (
                  <LimitOrderLadderPanel
                    pairAddress={pairAddr}
                    walletAddress={address}
                    escrowToken={escrowToken}
                    escrowDecimals={escrowDecimals}
                    token0Symbol={getTokenDisplaySymbol(token0 || 'token0')}
                    token1Symbol={getTokenDisplaySymbol(token1 || 'token1')}
                    disabled={!isWalletConnected || isPaused}
                    onPlaced={(ids) => {
                      if (ids.length > 0) setLastIndexedOrderId(Math.max(...ids))
                    }}
                  />
                )}
                {placeMode === 'single' && (
                  <>
                    <LimitOrderPlaceLimitHeading />
                    <LimitOrderBidAskSideSelector
                      idPrefix="limit-orders"
                      side={side}
                      onSideChange={handleSideChange}
                      bidLabel={`Bid (escrow ${getTokenDisplaySymbol(token1 || 'token1')})`}
                      askLabel={`Ask (escrow ${getTokenDisplaySymbol(token0 || 'token0')})`}
                    />
                    <LimitOrderPriceInputWithContext
                      side={side}
                      price={price}
                      onPriceChange={setPrice}
                      inputId={limitOrdersPriceInputId}
                      refToken1PerToken0={refToken1PerToken0}
                      refSource={refSource}
                      tapeHeadlineUsd={tapeHeadlineUsd}
                      token0Label={getTokenDisplaySymbol(token0 || 'token0')}
                      token1Label={getTokenDisplaySymbol(token1 || 'token1')}
                    />
                    <LimitOrderEscrowAmountField
                      escrowLabel={getTokenDisplaySymbol(escrowToken || '—')}
                      escrowDecimals={escrowDecimals}
                      amountHuman={amountHuman}
                      onAmountChange={onLimitAmountInputChange}
                      balanceQuery={escrowBalanceQuery}
                      onMax={onLimitAmountMax}
                      walletConnected={isWalletConnected}
                      maxContext="limit_place"
                      assetIsNativeUluna={escrowToken === 'uluna'}
                      escrowUsdNotionalApprox={escrowUsdNotionalApprox}
                    />
                    <LimitOrderExpiryField value={expiresAt} onChange={setExpiresAt} idPrefix="limit-orders-page" />
                    <LimitOrderAdvancedLimitSettings
                      open={limitAdvancedOpen}
                      onOpenChange={setLimitAdvancedOpen}
                      maxSteps={maxSteps}
                      onMaxStepsChange={setMaxSteps}
                      expiresAt={expiresAt}
                      onExpiresAtChange={setExpiresAt}
                      idPrefix="limit-orders-page"
                    />
                    {selectedPair && pairAddr.startsWith('terra1') && (
                      <LimitOrderPreSubmitSummary
                        placeSequenceMinUluna={limitPlaceMinUlunaFees}
                        refToken1PerToken0={refToken1PerToken0}
                        typedPrice={price}
                        effectiveFeeBps={effectiveFeeBps}
                        makerPlacementFeeBps={makerPlacementFeeBps}
                        feeLoading={limitFeeLoading}
                        feeError={limitFeeError}
                        data-testid="limits-page-pre-submit-summary"
                      />
                    )}
                    <button
                      type="button"
                      className="btn-primary btn-cta w-full"
                      disabled={
                        !isWalletConnected ||
                        placeMutation.isPending ||
                        !selectedPair ||
                        isPaused ||
                        !placeLimitCombinedOk
                      }
                      onClick={() => {
                        if (!isWalletConnected) openWalletModal()
                        else placeMutation.mutate()
                      }}
                    >
                      {!isWalletConnected
                        ? 'Connect Wallet'
                        : terraBroadcastPendingButtonLabel(
                            placeMutation.phase,
                            placeMutation.isPending,
                            'Place limit',
                            'Placing…'
                          )}
                    </button>
                    <TerraBroadcastPendingLink phase={placeMutation.phase} txHash={placeMutation.pendingTxHash} />
                    <LimitOrderEscrowPlaceGuardMessage
                      gate={placeLimitInlineGate}
                      data-testid="limits-page-place-guard"
                    />
                    {placeMutation.isError && (
                      <TxResultAlert type="error" message={(placeMutation.error as Error).message} />
                    )}
                    {placeMutation.isSuccess && (
                      <TxResultAlert type="success" message="Limit order placed." txHash={placeMutation.data} />
                    )}
                    {lastIndexedOrderId != null && (
                      <p className="text-xs font-mono" data-testid="last-placed-order-id">
                        Last indexed placement for your wallet: order #{lastIndexedOrderId}
                      </p>
                    )}
                  </>
                )}
              </div>

              <div className="card-neo !p-4 space-y-4">
                <h2 className="text-sm font-semibold uppercase tracking-wide">Cancel limit</h2>
                <p className="text-[11px] leading-snug" style={{ color: 'var(--ink-dim)' }}>
                  For your resting orders, use <strong>Edit</strong> / <strong>×</strong> on the order book above, or
                  enter an order id below.
                </p>
                <div>
                  <label className="label-neo" htmlFor={limitOrdersCancelOrderInputId}>
                    Order ID
                  </label>
                  <input
                    id={limitOrdersCancelOrderInputId}
                    className="input-neo w-full font-mono"
                    value={cancelOrderId}
                    onChange={(e) => setCancelOrderId(e.target.value)}
                    placeholder="e.g. 42"
                  />
                </div>
                <button
                  type="button"
                  className="btn-primary btn-cta w-full"
                  disabled={
                    !isWalletConnected ||
                    limitCancelMutation.isPending ||
                    !pairAddr ||
                    isPaused ||
                    cancelIdIndexedAsCancelled
                  }
                  onClick={() => {
                    if (!isWalletConnected) openWalletModal()
                    else submitCancelFromForm()
                  }}
                >
                  {!isWalletConnected
                    ? 'Connect Wallet'
                    : limitCancelMutation.isPending
                      ? 'Cancelling…'
                      : 'Cancel limit'}
                </button>
                {cancelIdIndexedAsCancelled && (
                  <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
                    This order id already has an indexed cancellation.
                  </p>
                )}
                {limitCancelMutation.isError && (
                  <TxResultAlert type="error" message={(limitCancelMutation.error as Error).message} />
                )}
                {limitCancelMutation.isSuccess && (
                  <TxResultAlert
                    type="success"
                    message="Cancel transaction submitted."
                    txHash={limitCancelMutation.data}
                  />
                )}
              </div>

              {pairAddr && address && (
                <div className="card-neo !p-4 space-y-2">
                  <LimitOrderMyPlacementsPanel
                    variant="page"
                    pairAddr={pairAddr}
                    pair={selectedPair}
                    walletAddress={address}
                    rows={myPlacements}
                    isLoading={placementsQuery.isLoading}
                    isWalletConnected={isWalletConnected}
                    isPairPaused={isPaused}
                    openWalletModal={openWalletModal}
                  />
                </div>
              )}

              {pairAddr && address && <WalletIndexerHistoryPanel walletAddress={address} pairAddress={pairAddr} />}
            </div>
          </LcdQueryGate>
        </div>
      </div>
    </div>
  )
}
