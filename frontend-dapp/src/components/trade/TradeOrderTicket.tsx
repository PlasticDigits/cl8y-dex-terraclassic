import { useMemo, useState, useEffect, useId, useRef, useCallback } from 'react'
import type { ReactNode } from 'react'
import type { UseMutationResult } from '@tanstack/react-query'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { useLimitOrderCancelMutation, type LimitOrderCancelInput } from '@/hooks/useLimitOrderCancelMutation'
import { useLimitOrderUpdatePriceMutation } from '@/hooks/useLimitOrderUpdatePriceMutation'
import { useWalletStore } from '@/hooks/useWallet'
import { usePairLimitCancellations } from '@/hooks/usePairLimitCancellations'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { placeLimitOrderWithAllowance, getPairPaused } from '@/services/terraclassic/pair'
import { useTradingBlacklist } from '@/hooks/useTradingBlacklist'
import {
  estimateLimitOrderPlaceSequenceUlunaFeesTotal,
  estimateUpdateLimitOrderPriceUlunaFeesTotal,
} from '@/services/terraclassic/transactions'
import { getTraderLimitPlacements } from '@/services/indexer/client'
import { sounds } from '@/lib/sounds'
import { TxResultAlert, Spinner } from '@/components/ui'
import { TerraBroadcastPendingLink } from '@/components/ui/TerraBroadcastPendingLink'
import { terraBroadcastPendingButtonLabel } from '@/utils/terraBroadcastUi'
import { assetInfoLabel, tokenAssetInfo, type IndexerPair, type IndexerTrade, type PairInfo } from '@/types'
import { formatNum, getDecimals, toRawAmount } from '@/utils/formatAmount'
import { evaluateLimitOrderEscrowPlaceGate } from '@/utils/limitOrderEscrowBalanceGate'
import { LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT } from '@/utils/limitOrderExpiry'
import { evaluateLimitOrderNativeGasPlaceGate } from '@/utils/limitOrderNativeGasBalanceGate'
import { evaluateLimitOrderPricePlaceGate } from '@/utils/limitOrderPricePlaceGate'
import { useLimitOrderPriceRefBundle } from '@/hooks/useLimitOrderPriceRefBundle'
import { warnIndexerPlacementPollFailed } from '@/utils/warnIndexerPlacementPollFailed'
import { orderIdHasIndexedCancellation } from '@/utils/limitOrderCancelUserMessage'
import { fetchCW20TokenInfo, getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { USER_INCIDENT_FAQ_HREF } from '@/components/legal/legalCopy'
import { useLimitEscrowMaxReapply } from '@/hooks/useLimitEscrowMaxReapply'
import { useLimitOrderForm } from '@/hooks/useLimitOrderForm'
import { useLimitOrderEscrowBalance } from '@/hooks/useLimitOrderEscrowBalance'
import { useNativeUlunaBalance } from '@/hooks/useNativeUlunaBalance'
import { LimitOrderAdvancedLimitSettings } from '@/components/trade/LimitOrderAdvancedLimitSettings'
import { LimitOrderBidAskSideSelector } from '@/components/trade/LimitOrderBidAskSideSelector'
import { LimitOrderEscrowAmountField } from '@/components/trade/LimitOrderEscrowAmountField'
import { LimitOrderEscrowPlaceGuardMessage } from '@/components/trade/LimitOrderEscrowPlaceGuardMessage'
import { LimitOrderExpiryField } from '@/components/trade/LimitOrderExpiryField'
import { LimitOrderMyPlacementsPanel } from '@/components/trade/LimitOrderMyPlacementsPanel'
import { LimitOrderPreSubmitSummary } from '@/components/trade/LimitOrderPreSubmitSummary'
import { LimitOrderPlaceLimitHeading, LimitOrderPriceInputWithContext } from '@/components/trade/LimitOrderPriceField'
import { useLimitOrderMakerFeeRates } from '@/hooks/useLimitOrderMakerFeeRates'
import { useTradeBestBookPrices } from '@/hooks/useTradeBestBookPrices'
import { useLimitBookInfinite } from '@/hooks/useLimitBookInfinite'
import { describeLimitCrossingBlocker } from '@/utils/limitOrderNonCrossing'
import { flattenLimitBookPages, resolveLimitInsertHintAfter } from '@/utils/limitBookInsertHint'
import { escrowAmountUsdAnchorNotional, parsePositivePriceHuman } from '@/utils/limitOrderPriceReference'
import { TradeMarketOrderPanel } from '@/components/trade/TradeMarketOrderPanel'
import type { LimitBookEditContext, LimitBookTicketDraft } from '@/types/limitBookTicketDraft'
import {
  buildLimitBookEditContext,
  isPriceOnlyLimitEdit,
  LIMIT_EDIT_NON_PRICE_CHANGE_MESSAGE,
} from '@/utils/limitOrderPriceEdit'
import { tradeDirectionSideLabels } from '@/utils/tradeDirectionSideLabels'

function TicketSection({
  eyebrow,
  title,
  children,
  tone = 'default',
}: {
  eyebrow?: string
  title: string
  children: ReactNode
  tone?: 'default' | 'action' | 'manage'
}) {
  const borderColor =
    tone === 'action' ? 'rgba(251, 146, 60, 0.22)' : tone === 'manage' ? 'rgba(148, 163, 184, 0.16)' : 'var(--line)'
  return (
    <section
      className="rounded-2xl border p-3 space-y-3"
      style={{
        borderColor,
        background:
          tone === 'action'
            ? 'linear-gradient(180deg, rgba(251, 146, 60, 0.07), rgba(255, 255, 255, 0.02))'
            : 'rgba(255, 255, 255, 0.025)',
      }}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          {eyebrow && (
            <p className="text-[9px] font-semibold uppercase tracking-[0.16em]" style={{ color: 'var(--ink-subtle)' }}>
              {eyebrow}
            </p>
          )}
          <h3 className="text-xs font-semibold uppercase tracking-wide" style={{ color: 'var(--ink)' }}>
            {title}
          </h3>
        </div>
      </div>
      {children}
    </section>
  )
}

function TicketStat({ label, value, tone }: { label: string; value: ReactNode; tone?: 'bid' | 'ask' }) {
  const color = tone === 'bid' ? 'var(--color-positive)' : tone === 'ask' ? 'var(--color-negative)' : 'var(--ink)'
  return (
    <div className="rounded-xl border border-white/10 px-2.5 py-2" style={{ background: 'rgba(255,255,255,0.025)' }}>
      <div className="text-[9px] font-semibold uppercase tracking-wide" style={{ color: 'var(--ink-subtle)' }}>
        {label}
      </div>
      <div className="mt-0.5 font-mono text-[11px] tabular-nums truncate" style={{ color }}>
        {value}
      </div>
    </div>
  )
}

export type TradeOrderTicketProps = {
  pairAddr: string
  pairs: PairInfo[]
  pairsLoading: boolean
  indexerPair?: IndexerPair | null
  latestTrade?: IndexerTrade | null
  tapeHeadlineUsd?: string | null
  /**
   * When set (e.g. from `TradePage` with `OrderBookPanel`), book rows and ticket share one cancel mutation
   * ([GitLab #162](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/162)).
   */
  cancelLimitOrderMutation?: UseMutationResult<string, Error, LimitOrderCancelInput, unknown>
  /** Incrementing key so the ticket applies a book-driven prefill even when fields match the prior draft. */
  limitBookDraftKey?: number
  limitBookDraft?: LimitBookTicketDraft | null
  onLimitBookDraftConsumed?: () => void
}

type TradeOrderTicketContentProps = TradeOrderTicketProps & {
  cancelLimitOrderMutation: UseMutationResult<string, Error, LimitOrderCancelInput, unknown>
}

/**
 * Trade workspace order ticket: **Market** (taker swap + slippage + hybrid quote) and **Limit** (resting book),
 * plus cancel + “my placements” ([GitLab #152](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/152)).
 */
function TradeOrderTicketContent({
  pairAddr,
  pairs,
  pairsLoading,
  indexerPair,
  latestTrade,
  tapeHeadlineUsd,
  cancelLimitOrderMutation,
  limitBookDraftKey = 0,
  limitBookDraft,
  onLimitBookDraftConsumed,
}: TradeOrderTicketContentProps) {
  const address = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)
  const wallet = getConnectedWallet()
  const isWalletConnected = !!address && !!wallet
  const queryClient = useQueryClient()
  const limitPriceInputId = useId()
  const cancelLimitOrderInputId = useId()
  const placementsAnchorRef = useRef<HTMLDivElement>(null)
  const [highlightPlacementOrderId, setHighlightPlacementOrderId] = useState<number | null>(null)

  const [side, setSide] = useState<'bid' | 'ask'>('bid')
  const [orderTab, setOrderTab] = useState<'limit' | 'market'>('limit')
  const orderTypeTabBaseId = useId()
  const limitOrderTabId = `${orderTypeTabBaseId}-limit-tab`
  const marketOrderTabId = `${orderTypeTabBaseId}-market-tab`
  const limitOrderPanelId = `${orderTypeTabBaseId}-limit-panel`
  const marketOrderPanelId = `${orderTypeTabBaseId}-market-panel`
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
  const [editContext, setEditContext] = useState<LimitBookEditContext | null>(null)
  const [editHintAfterOrderId, setEditHintAfterOrderId] = useState<number | null>(null)

  const selectedPair = useMemo(() => pairs.find((p) => p.contract_addr === pairAddr), [pairs, pairAddr])

  const { refToken1PerToken0, refSource, refResolutionLoading, refResolutionError } = useLimitOrderPriceRefBundle({
    pairAddr,
    selectedPair,
    indexerPair,
    latestTrade,
  })

  const token0 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[0]) : ''
  const token1 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[1]) : ''
  const escrowToken = side === 'bid' ? token1 : token0
  const escrowDecimals = escrowToken ? getDecimals(tokenAssetInfo(escrowToken)) : 6
  const escrowBalanceQuery = useLimitOrderEscrowBalance(address, escrowToken)
  const nativeUlunaQuery = useNativeUlunaBalance(address)

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
  const limitPlaceMinUlunaFees = useMemo(() => estimateLimitOrderPlaceSequenceUlunaFeesTotal(), [])
  const updatePriceMinUlunaFees = useMemo(() => estimateUpdateLimitOrderPriceUlunaFeesTotal(), [])

  const {
    effectiveFeeBps,
    makerPlacementFeeBps,
    feeLoading: limitFeeLoading,
    feeError: limitFeeError,
  } = useLimitOrderMakerFeeRates(pairAddr, address ?? undefined)

  const placementsQuery = useQuery({
    queryKey: ['limitPlacements', pairAddr, address],
    queryFn: () => getTraderLimitPlacements(address!, { pair: pairAddr, limit: 100 }),
    enabled: pairAddr.startsWith('terra1') && !!address,
  })

  const cancellationsQuery = usePairLimitCancellations(pairAddr)

  const pausedQuery = useQuery({
    queryKey: ['pairPaused', pairAddr],
    queryFn: () => getPairPaused(pairAddr),
    enabled: pairAddr.startsWith('terra1'),
    staleTime: 15_000,
  })

  const isPaused = pausedQuery.data?.paused === true

  const token0Addr =
    selectedPair && 'token' in selectedPair.asset_infos[0] ? selectedPair.asset_infos[0].token.contract_addr : null
  const token1Addr =
    selectedPair && 'token' in selectedPair.asset_infos[1] ? selectedPair.asset_infos[1].token.contract_addr : null
  const tradingBlacklist = useTradingBlacklist({
    wallet: address,
    token0: token0Addr,
    token1: token1Addr,
    pairAddress: pairAddr,
    enabled: pairAddr.startsWith('terra1'),
  })
  const isTradeBlocked = isPaused || tradingBlacklist.blocked

  const { bestBid, bestAsk, isLoading: bestBookLoading } = useTradeBestBookPrices(pairAddr)
  const limitBookQuery = useLimitBookInfinite(pairAddr, side)
  const placeInsertHintAfter = useMemo(() => {
    const { orders, hasMore } = flattenLimitBookPages(limitBookQuery.data?.pages)
    return resolveLimitInsertHintAfter(side, price, orders, { hasMore })
  }, [limitBookQuery.data?.pages, side, price])

  const crossingBlocker = useMemo(
    () => describeLimitCrossingBlocker(side, price, bestBid, bestAsk),
    [side, price, bestBid, bestAsk]
  )

  const placePriceGate = useMemo(
    () =>
      evaluateLimitOrderPricePlaceGate(side, price, refToken1PerToken0, {
        refResolutionLoading,
        refResolutionError,
      }),
    [side, price, refToken1PerToken0, refResolutionLoading, refResolutionError]
  )

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
    !crossingBlocker &&
    !expiryPastBlocker &&
    !editContext

  const priceOnlyEdit = useMemo(
    () =>
      isPriceOnlyLimitEdit(editContext, {
        side,
        price,
        amountHuman,
        expiresAt,
      }),
    [editContext, side, price, amountHuman, expiresAt]
  )

  const editNonPriceChanged = useMemo(() => {
    if (!editContext) return false
    if (editContext.side !== side) return true
    if (editContext.amountHuman !== amountHuman.trim()) return true
    if ((expiresAt ?? null) !== editContext.expiresAt) return true
    return false
  }, [editContext, side, amountHuman, expiresAt])

  const updatePriceNativeGasGate = useMemo(
    () =>
      evaluateLimitOrderNativeGasPlaceGate(
        '',
        escrowDecimals,
        {
          data: nativeUlunaQuery.data,
          isLoading: nativeUlunaQuery.isLoading,
          isError: nativeUlunaQuery.isError,
        },
        updatePriceMinUlunaFees
      ),
    [
      escrowDecimals,
      nativeUlunaQuery.data,
      nativeUlunaQuery.isLoading,
      nativeUlunaQuery.isError,
      updatePriceMinUlunaFees,
    ]
  )

  const updatePriceCombinedOk =
    priceOnlyEdit && placePriceGate.canPlaceLimit && !crossingBlocker && updatePriceNativeGasGate.canPlaceLimit

  const placeLimitInlineGate = useMemo(() => {
    if (expiryPastBlocker) {
      return { canPlaceLimit: false, userMessage: expiryPastBlocker, tone: 'warning' as const }
    }
    if (crossingBlocker) {
      return { canPlaceLimit: false, userMessage: crossingBlocker, tone: 'warning' as const }
    }
    if (!placePriceGate.canPlaceLimit) {
      return placePriceGate
    }
    return placeEscrowGate.userMessage ? placeEscrowGate : placeNativeGasGate
  }, [expiryPastBlocker, crossingBlocker, placePriceGate, placeEscrowGate, placeNativeGasGate])

  const myPlacements = useMemo(() => placementsQuery.data ?? [], [placementsQuery.data])

  const parsedCancelOrderId = parseInt(cancelOrderId, 10)
  const cancelIdIndexedAsCancelled =
    Number.isFinite(parsedCancelOrderId) &&
    parsedCancelOrderId >= 1 &&
    orderIdHasIndexedCancellation(cancellationsQuery.data ?? [], parsedCancelOrderId)

  const cancelMutation = cancelLimitOrderMutation
  const updatePriceMutation = useLimitOrderUpdatePriceMutation(pairAddr, address ?? undefined)

  const clearEditContext = useCallback(() => {
    setEditContext(null)
    setEditHintAfterOrderId(null)
  }, [])

  const placeMutation = useTerraBroadcastMutation({
    toastSuccess: 'Limit order placed.',
    mutationFn: async () => {
      if (!address) throw new Error('Connect wallet')
      if (!selectedPair) throw new Error('Select a pair')
      if (!escrowToken.startsWith('terra1')) throw new Error('Escrow token must be CW20')
      const cross = describeLimitCrossingBlocker(side, price, bestBid, bestAsk)
      if (cross) throw new Error(cross)
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
      queryClient.invalidateQueries({ queryKey: ['tradeBestBook', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['limitOrderPricePoolRef', pairAddr] })
      setLastIndexedOrderId(null)
      const addr = pairAddr
      const walletAddr = address
      if (!addr.startsWith('terra1') || !walletAddr) return
      for (let i = 0; i < 24; i++) {
        await new Promise((r) => setTimeout(r, 500))
        try {
          const rows = await getTraderLimitPlacements(walletAddr, { pair: addr, limit: 100 })
          const maxId = rows.reduce((m, r) => Math.max(m, r.order_id), 0)
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

  const submitUpdateLimitPrice = () => {
    if (!isWalletConnected) {
      openWalletModal()
      return
    }
    if (!editContext) return
    const cross = describeLimitCrossingBlocker(side, price, bestBid, bestAsk)
    if (cross) {
      updatePriceMutation.reset()
      return
    }
    updatePriceMutation.mutate(
      {
        orderId: editContext.orderId,
        price,
        maxAdjustSteps: maxSteps,
        hintAfterOrderId: editHintAfterOrderId,
      },
      {
        onSuccess: () => {
          clearEditContext()
          placeMutation.reset()
        },
      }
    )
  }

  const submitCancelFromForm = () => {
    if (!isWalletConnected) {
      openWalletModal()
      return
    }
    const id = parseInt(cancelOrderId, 10)
    cancelMutation.mutate(id, {
      onSuccess: () => {
        setCancelOrderId('')
        setLastIndexedOrderId(null)
      },
    })
  }

  const focusLimitPriceField = () => {
    requestAnimationFrame(() => {
      const el = document.getElementById(limitPriceInputId)
      if (el instanceof HTMLInputElement) {
        el.focus()
        el.select()
      }
    })
  }

  /** Scroll to **My limits** or the active row for the last indexed `order_id` ([GitLab #161](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/161)). */
  const onViewPlacedLimitOrder = () => {
    sounds.playButtonPress()
    const oid = lastIndexedOrderId
    if (oid != null) {
      const row = document.querySelector<HTMLElement>(`[data-testid="trade-placement-active-${oid}"]`)
      if (row) {
        row.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
        setHighlightPlacementOrderId(oid)
        window.setTimeout(() => setHighlightPlacementOrderId(null), 2600)
        return
      }
    }
    placementsAnchorRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
  }

  const onPlaceAnotherLimit = () => {
    sounds.playButtonPress()
    placeMutation.reset()
    updatePriceMutation.reset()
    clearEditContext()
    resetLimitEscrowAmount()
    setPrice('1')
    setExpiresAt(null)
    setMaxSteps(LIMIT_ORDER_MAX_ADJUST_STEPS_DEFAULT)
    setLimitAdvancedOpen(false)
    setLastIndexedOrderId(null)
    setCancelOrderId('')
    setHighlightPlacementOrderId(null)
    focusLimitPriceField()
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

  useEffect(() => {
    if (!limitBookDraft || limitBookDraftKey < 1) return
    setOrderTab('limit')
    setSide(limitBookDraft.side)
    setPrice(limitBookDraft.price)
    setLimitEscrowAmountFromDraft(limitBookDraft.amountHuman)
    setEditContext(buildLimitBookEditContext(limitBookDraft))
    setEditHintAfterOrderId(limitBookDraft.hintAfterOrderId ?? null)
    onLimitBookDraftConsumed?.()
  }, [limitBookDraftKey, limitBookDraft, onLimitBookDraftConsumed, setLimitEscrowAmountFromDraft])

  if (pairsLoading) {
    return (
      <div className="flex justify-center py-12 card-glass">
        <Spinner />
      </div>
    )
  }

  const token0Display = indexerPair?.asset_0.symbol ?? getTokenDisplaySymbol(token0 || 'token0')
  const token1Display = indexerPair?.asset_1.symbol ?? getTokenDisplaySymbol(token1 || 'token1')
  const { bidLabel: directionBidLabel, askLabel: directionAskLabel } = tradeDirectionSideLabels(token0Display)
  const sideAction =
    side === 'bid'
      ? { verb: 'Buy', receive: token0Display, pay: token1Display, tone: 'bid' as const }
      : { verb: 'Sell', receive: token0Display, pay: token0Display, tone: 'ask' as const }
  const walletLabel = isWalletConnected && address ? `${address.slice(0, 8)}…${address.slice(-6)}` : 'Connect wallet'
  const bestBidLabel = bestBookLoading ? '…' : (bestBid ?? '—')
  const bestAskLabel = bestBookLoading ? '…' : (bestAsk ?? '—')

  return (
    <div className="flex flex-col h-full min-h-0 card-glass !p-0">
      <div
        className="p-4 border-b border-white/10"
        style={{
          background:
            'radial-gradient(circle at 20% 0%, rgba(251, 146, 60, 0.18), transparent 34%), rgba(255,255,255,0.025)',
        }}
      >
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <p className="text-[9px] font-semibold uppercase tracking-[0.18em]" style={{ color: 'var(--ink-subtle)' }}>
              Order ticket
            </p>
            <h3 className="mt-1 text-base font-semibold font-heading truncate" style={{ color: 'var(--ink)' }}>
              {selectedPair ? `${sideAction.verb} ${sideAction.receive}` : 'Select a pair'}
            </h3>
            <p className="mt-1 text-[10px] leading-snug hidden lg:block" style={{ color: 'var(--ink-dim)' }}>
              {selectedPair
                ? `${sideAction.pay} funds the order. Resting limits appear in the book; market orders take available liquidity.`
                : 'Choose a trading pair from the selector to place orders.'}
            </p>
          </div>
          <button
            type="button"
            onClick={() => {
              if (!isWalletConnected) openWalletModal()
            }}
            className="shrink-0 rounded-full border px-2.5 py-1 text-[9px] font-semibold uppercase tracking-wide"
            style={{
              color: isWalletConnected ? 'var(--color-positive)' : 'var(--ink-subtle)',
              borderColor: isWalletConnected ? 'rgba(34,197,94,0.35)' : 'var(--line)',
            }}
            title={isWalletConnected ? `Connected wallet ${address}` : 'Wallet is not connected'}
          >
            {walletLabel}
          </button>
        </div>

        {selectedPair && (
          <div className="mt-3 grid grid-cols-2 gap-2">
            <TicketStat label="Base" value={token0Display} />
            <TicketStat label="Quote" value={token1Display} />
          </div>
        )}
      </div>

      <div className="flex flex-1 flex-col gap-3 min-h-0 overflow-y-auto p-4">
        {selectedPair && isPaused && (
          <div className="alert-error text-xs space-y-2" role="alert">
            <p>
              Pair is paused — swaps, limit place, cancel, and parked-expiry claim are blocked until governance
              unpauses. Your funds and escrow remain in the pair contract.
            </p>
            <a
              className="underline text-[10px]"
              href={USER_INCIDENT_FAQ_HREF}
              target="_blank"
              rel="noopener noreferrer"
            >
              What happens during an incident?
            </a>
          </div>
        )}

        {selectedPair && tradingBlacklist.blocked && tradingBlacklist.message && (
          <div className="alert-error text-xs space-y-2" role="alert">
            <p>{tradingBlacklist.message}</p>
            <p className="text-[10px] opacity-90">
              Restrictions are enforced on-chain by governance. Funds remain recoverable when the restriction is lifted.
            </p>
          </div>
        )}

        {selectedPair && pairAddr.startsWith('terra1') && (
          <div
            className="grid grid-cols-2 gap-1 rounded-2xl border border-white/10 p-1"
            style={{ background: 'rgba(255,255,255,0.025)' }}
            role="tablist"
            aria-label="Order type"
          >
            <button
              type="button"
              id={limitOrderTabId}
              role="tab"
              aria-selected={orderTab === 'limit'}
              aria-controls={limitOrderPanelId}
              data-testid="trade-order-tab-limit"
              className={`tab-glass !text-xs !px-3 !py-2 w-full justify-center ${orderTab === 'limit' ? 'tab-glass-active' : 'tab-glass-inactive'}`}
              onClick={() => {
                sounds.playButtonPress()
                setOrderTab('limit')
              }}
            >
              Limit
            </button>
            <button
              type="button"
              id={marketOrderTabId}
              role="tab"
              aria-selected={orderTab === 'market'}
              aria-controls={marketOrderPanelId}
              data-testid="trade-order-tab-market"
              className={`tab-glass !text-xs !px-3 !py-2 w-full justify-center ${orderTab === 'market' ? 'tab-glass-active' : 'tab-glass-inactive'}`}
              onClick={() => {
                sounds.playButtonPress()
                setOrderTab('market')
              }}
            >
              Market
            </button>
          </div>
        )}

        <TicketSection eyebrow={orderTab === 'limit' ? 'Maker side' : 'Taker side'} title="Choose direction">
          <LimitOrderBidAskSideSelector
            idPrefix="trade-ticket"
            compact
            side={side}
            onSideChange={handleSideChange}
            bidLabel={directionBidLabel}
            askLabel={directionAskLabel}
          />
          {selectedPair && pairAddr.startsWith('terra1') && (
            <div className="grid grid-cols-2 gap-2">
              <TicketStat label="Best bid" value={bestBidLabel} tone="bid" />
              <TicketStat label="Best ask" value={bestAskLabel} tone="ask" />
            </div>
          )}
          <p className="text-[10px] leading-snug hidden lg:block" style={{ color: 'var(--ink-dim)' }}>
            Prices are quoted in {token1Display} per {token0Display}. Buy limits should sit below the reference; sell
            limits above it.
          </p>
        </TicketSection>

        {orderTab === 'market' && selectedPair && (
          <div role="tabpanel" id={marketOrderPanelId} aria-labelledby={marketOrderTabId}>
            <TicketSection eyebrow="Take liquidity" title={`${sideAction.verb} now`} tone="action">
              <TradeMarketOrderPanel
                pairAddr={pairAddr}
                selectedPair={selectedPair}
                side={side}
                isPaused={isTradeBlocked}
              />
            </TicketSection>
          </div>
        )}

        {orderTab === 'limit' && (
          <div role="tabpanel" id={limitOrderPanelId} aria-labelledby={limitOrderTabId}>
            {pairAddr && address && (
              <div
                ref={placementsAnchorRef}
                data-testid="trade-ticket-placements-anchor"
                className="scroll-mt-4 outline-none mb-3"
                tabIndex={-1}
              >
                <LimitOrderMyPlacementsPanel
                  variant="compact"
                  pairAddr={pairAddr}
                  pair={selectedPair}
                  walletAddress={address}
                  rows={myPlacements}
                  isLoading={placementsQuery.isLoading}
                  isWalletConnected={isWalletConnected}
                  isPairPaused={isPaused}
                  claimsDisabled={tradingBlacklist.blocked}
                  cancelDisabled={tradingBlacklist.blocked}
                  openWalletModal={openWalletModal}
                  cancelLimitOrderMutation={cancelMutation}
                  cancellations={cancellationsQuery.data ?? []}
                  highlightOrderId={highlightPlacementOrderId}
                />
              </div>
            )}
            <TicketSection eyebrow="Resting order" title={`${sideAction.verb} at your price`} tone="action">
              <LimitOrderPlaceLimitHeading compact />
              <LimitOrderPriceInputWithContext
                side={side}
                price={price}
                onPriceChange={setPrice}
                inputId={limitPriceInputId}
                refToken1PerToken0={refToken1PerToken0}
                refSource={refSource}
                tapeHeadlineUsd={tapeHeadlineUsd}
                token0Label={token0Display}
                token1Label={token1Display}
                compact
              />
              <LimitOrderEscrowAmountField
                compact
                escrowLabel={sideAction.pay}
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
              <LimitOrderExpiryField compact value={expiresAt} onChange={setExpiresAt} idPrefix="trade-ticket" />
              <LimitOrderAdvancedLimitSettings
                compact
                open={limitAdvancedOpen}
                onOpenChange={setLimitAdvancedOpen}
                maxSteps={maxSteps}
                onMaxStepsChange={setMaxSteps}
                expiresAt={expiresAt}
                onExpiresAtChange={setExpiresAt}
                idPrefix="trade-ticket"
              />
              {selectedPair && pairAddr.startsWith('terra1') && (
                <LimitOrderPreSubmitSummary
                  compact
                  placeSequenceMinUluna={limitPlaceMinUlunaFees}
                  refToken1PerToken0={refToken1PerToken0}
                  typedPrice={price}
                  effectiveFeeBps={effectiveFeeBps}
                  makerPlacementFeeBps={makerPlacementFeeBps}
                  feeLoading={limitFeeLoading}
                  feeError={limitFeeError}
                  data-testid="trade-limit-pre-submit-summary"
                />
              )}
              {editContext && (
                <p
                  className="text-[10px] leading-snug rounded-lg border border-white/10 px-2.5 py-2"
                  style={{ color: 'var(--ink-dim)' }}
                  data-testid="trade-limit-edit-context"
                >
                  Editing order <span className="font-mono">#{editContext.orderId}</span>
                  {priceOnlyEdit
                    ? ' — change price and tap Update price (one tx, no maker fee).'
                    : editNonPriceChanged
                      ? ` — ${LIMIT_EDIT_NON_PRICE_CHANGE_MESSAGE}`
                      : ' — adjust price to update in one tx.'}
                </p>
              )}
            </TicketSection>
            <div
              className="sticky bottom-0 z-10 -mx-4 px-4 py-3 space-y-2 border-t border-white/10"
              style={{ background: 'color-mix(in srgb, var(--card) 92%, transparent)' }}
              data-testid="trade-limit-submit-sticky"
            >
              <button
                type="button"
                data-testid={priceOnlyEdit ? 'trade-limit-update-price-submit' : 'trade-limit-submit'}
                className="btn-primary btn-cta w-full !text-xs"
                disabled={
                  priceOnlyEdit
                    ? updatePriceMutation.isPending ||
                      !selectedPair ||
                      isTradeBlocked ||
                      (isWalletConnected && !updatePriceCombinedOk)
                    : placeMutation.isPending ||
                      !selectedPair ||
                      isTradeBlocked ||
                      editNonPriceChanged ||
                      (isWalletConnected && !placeLimitCombinedOk)
                }
                onClick={() => {
                  if (!isWalletConnected) openWalletModal()
                  else if (priceOnlyEdit) submitUpdateLimitPrice()
                  else placeMutation.mutate()
                }}
              >
                {!isWalletConnected
                  ? 'Connect Wallet'
                  : priceOnlyEdit
                    ? updatePriceMutation.isPending
                      ? 'Updating…'
                      : 'Update price'
                    : terraBroadcastPendingButtonLabel(
                        placeMutation.phase,
                        placeMutation.isPending,
                        'Place limit',
                        'Placing…'
                      )}
              </button>
              <TerraBroadcastPendingLink phase={placeMutation.phase} txHash={placeMutation.pendingTxHash} />
              {priceOnlyEdit && updatePriceNativeGasGate.userMessage && (
                <LimitOrderEscrowPlaceGuardMessage
                  gate={updatePriceNativeGasGate}
                  data-testid="trade-limit-update-price-guard"
                />
              )}
              {!priceOnlyEdit && (
                <LimitOrderEscrowPlaceGuardMessage gate={placeLimitInlineGate} data-testid="trade-limit-place-guard" />
              )}
              {updatePriceMutation.isError && (
                <TxResultAlert type="error" message={(updatePriceMutation.error as Error).message} />
              )}
              {updatePriceMutation.isSuccess && (
                <TxResultAlert type="success" message="Limit order price updated." txHash={updatePriceMutation.data} />
              )}
              {placeMutation.isError && <TxResultAlert type="error" message={(placeMutation.error as Error).message} />}
              {placeMutation.isSuccess && (
                <TxResultAlert type="success" message="Limit order placed." txHash={placeMutation.data} />
              )}
            </div>
            {placeMutation.isSuccess && (
              <div className="space-y-2" data-testid="trade-limit-post-place-actions">
                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    data-testid="trade-limit-view-order-btn"
                    className="btn-primary btn-cta flex-1 min-w-[7.5rem] !text-[10px] !py-2 !px-3"
                    onClick={onViewPlacedLimitOrder}
                  >
                    View order
                  </button>
                  <button
                    type="button"
                    data-testid="trade-limit-place-another-btn"
                    className="btn-muted flex-1 min-w-[7.5rem] !text-[10px] !py-2 !px-3"
                    onClick={onPlaceAnotherLimit}
                  >
                    Place another
                  </button>
                </div>
                {lastIndexedOrderId == null && (
                  <p className="text-[10px] leading-snug" style={{ color: 'var(--ink-dim)' }}>
                    If your new row is not listed yet, the indexer is still catching up — tap{' '}
                    <span className="font-medium" style={{ color: 'var(--ink)' }}>
                      View order
                    </span>{' '}
                    again after a moment to jump to the highlighted line in <strong>My limits</strong> below.
                  </p>
                )}
              </div>
            )}
            {lastIndexedOrderId != null && (
              <p className="text-[10px] font-mono" data-testid="trade-last-placed-order-id">
                Last indexed: #{lastIndexedOrderId}
              </p>
            )}
          </div>
        )}

        <details
          className="rounded-2xl border p-3 space-y-3 group"
          style={{ borderColor: 'rgba(148, 163, 184, 0.16)' }}
        >
          <summary className="text-xs font-semibold uppercase tracking-wide cursor-pointer list-none">
            Cancel by order ID
            <span className="block text-[9px] font-normal normal-case opacity-70 mt-0.5">
              Advanced — use Cancel on open limits above
            </span>
          </summary>
          <div className="space-y-3 pt-1">
            <div>
              <label className="label-glass" htmlFor={cancelLimitOrderInputId}>
                Order ID
              </label>
              <input
                id={cancelLimitOrderInputId}
                className="input-glass w-full font-mono text-sm"
                value={cancelOrderId}
                onChange={(e) => setCancelOrderId(e.target.value)}
                placeholder="Order ID"
              />
            </div>
            <button
              type="button"
              data-testid="trade-cancel-submit"
              className="btn-primary btn-cta w-full !text-xs"
              disabled={
                cancelMutation.isPending ||
                !pairAddr ||
                isTradeBlocked ||
                (isWalletConnected && cancelIdIndexedAsCancelled)
              }
              onClick={() => {
                if (!isWalletConnected) openWalletModal()
                else submitCancelFromForm()
              }}
            >
              {!isWalletConnected ? 'Connect Wallet' : cancelMutation.isPending ? 'Cancelling…' : 'Cancel'}
            </button>
            {cancelIdIndexedAsCancelled && (
              <p className="text-[10px]" style={{ color: 'var(--ink-dim)' }}>
                This order id already has an indexed cancellation.
              </p>
            )}
            {cancelMutation.isError && <TxResultAlert type="error" message={(cancelMutation.error as Error).message} />}
            {cancelMutation.isSuccess && (
              <TxResultAlert type="success" message="Cancel submitted." txHash={cancelMutation.data} />
            )}
          </div>
        </details>
      </div>
    </div>
  )
}

function TradeOrderTicketWithLocalCancel(props: Omit<TradeOrderTicketProps, 'cancelLimitOrderMutation'>) {
  const address = useWalletStore((s) => s.address)
  const cancelLimitOrderMutation = useLimitOrderCancelMutation(props.pairAddr, address ?? undefined)
  return <TradeOrderTicketContent {...props} cancelLimitOrderMutation={cancelLimitOrderMutation} />
}

export function TradeOrderTicket(props: TradeOrderTicketProps) {
  if (props.cancelLimitOrderMutation) {
    return <TradeOrderTicketContent {...(props as TradeOrderTicketContentProps)} />
  }
  const { cancelLimitOrderMutation, ...rest } = props
  void cancelLimitOrderMutation
  return <TradeOrderTicketWithLocalCancel {...rest} />
}
