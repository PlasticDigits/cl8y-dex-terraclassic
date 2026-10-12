import { slippagePercentToMaxSpread } from '@/utils/slippageConstraint'
import { AmountBalanceActions } from '@/components/common/AmountBalanceActions'
import { SlippageProtectionPresets } from '@/components/common/SlippageProtectionPresets'
import { TerraClassicTxFeeHint } from '@/components/common/TerraClassicTxFeeHint'
import { SwapPayAcquireGuidanceBanner } from '@/components/swap/SwapPayAcquireGuidanceBanner'
import { SwapPreSubmitSummary } from '@/components/swap/SwapPreSubmitSummary'
import { LimitOrderEscrowAmountField } from '@/components/trade/LimitOrderEscrowAmountField'
import { LimitOrderEscrowPlaceGuardMessage } from '@/components/trade/LimitOrderEscrowPlaceGuardMessage'
import { TradeMarketSubmitChrome, type TradeMarketSubmitChromeModel } from '@/components/trade/TradeTicketSubmitFooter'
import { Spinner } from '@/components/ui'
import { useAssetDecimals } from '@/hooks/useAssetDecimals'
import { useCommunityTaxPreviewDebit, useCommunityTaxSellBps } from '@/hooks/useCommunityTaxSellBps'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'
import { useLimitOrderEscrowBalance } from '@/hooks/useLimitOrderEscrowBalance'
import { useNativeUlunaBalance } from '@/hooks/useNativeUlunaBalance'
import { useRouterTaxFunding } from '@/hooks/useRouterTaxFunding'
import { useSubmitAlignedSimQuote } from '@/hooks/useSubmitAlignedSimQuote'
import { useSwapPayAcquireGuidance } from '@/hooks/useSwapPayAcquireGuidance'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { LocalQuoteTokenMetadata } from '@/hooks/useTokenDisplayInfo'
import { useWalletStore } from '@/hooks/useWallet'
import { sounds } from '@/lib/sounds'
import { executeMultiHopSwap, type SwapOperation } from '@/services/terraclassic/router'
import { tryEstimateSwapNetworkFee, estimateTradeMarketNetworkFeeUluna } from '@/services/terraclassic/swapNetworkFee'
import { type SwapRoutePreflightSpread } from '@/services/terraclassic/swapRoutePreflight'
import { hybridFromSingleHopIndexerOps } from '@/services/terraclassic/swapRouting'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { useDexStore } from '@/stores/dex'
import { assetInfoLabel, type HybridSwapParams, type IndexerRouteQuoteKind, type PairInfo } from '@/types'
import { resolveCommunityTaxPreviewQuery } from '@/utils/communityTaxPreviewQuery'
import { quoteCw20ViaRouteSolve } from '@/utils/cw20RouteSolveQuote'
import { isDecimalAmountDraft, isPositiveDecimalAmount, tryParseBigInt } from '@/utils/decimalAmountInput'
import { DIRECT_HYBRID_AMOUNT_RECONCILED_COPY, quoteDisclosureForIndexerKind } from '@/utils/directHybridQuote'
import { formatTokenAmount, fromRawAmount, toRawAmount } from '@/utils/formatAmount'
import { humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'
import { useQuoteErrorRecovery } from '@/hooks/useQuoteErrorRecovery'
import { useQuotePricingDataPending } from '@/hooks/useQuotePricingDataPending'
import { QuoteDataLoadingProgress } from '@/components/QuoteDataLoadingProgress'
import { isExpiredLocalQuote, localQuoteStaleTime, localQuoteRefetchInterval } from '@/utils/localQuoteFreshness'
import { isIndexerLocalQuoteRejection } from '@/utils/indexerErrors'
import {
  evaluateLimitOrderEscrowPlaceGate,
  LIMIT_ORDER_ESCROW_MSG_INSUFFICIENT,
} from '@/utils/limitOrderEscrowBalanceGate'
import { evaluateMarketSwapNativeGasPlaceGate } from '@/utils/limitOrderNativeGasBalanceGate'
import { assertLocalRouteSubmit, LOCAL_ROUTE_UNAVAILABLE } from '@/utils/localRouteQuote'
import { computeMaxSpendableHumanAmount } from '@/utils/maxSpendableAmount'
import { getNetworkBadgeCopy } from '@/utils/networkDisplay'
import { useNumberDisplayMode } from '@/utils/numberDisplayPreference'
import {
  assertSubmitHybridAligned,
  assertSubmitQuotePayRawAligned,
  shouldShowSimReceiveCalculating,
  SIM_QUOTE_DEBOUNCE_MS,
} from '@/utils/quoteDebounce'
import { resolveSwapRoutePairAddresses } from '@/utils/resolveSwapRoutePairAddresses'
import { getDirectHybridBookSplit, getIndexerHybridExecutionSummary } from '@/utils/swapDisclosure'
import {
  SWAP_FUNDED_HIGH_IMPACT_PCT,
  acquireGuidanceShowsQuoteOnly,
  acquireReduceHiddenForBroadcastPhase,
} from '@/utils/swapPayAcquireGuidance'
import { isTheaterRouteQuote, swapRouteSlippageBlocksSubmit } from '@/utils/swapQuoteAmountScale'
import { computeSwapRouteDisplay } from '@/utils/swapRouteDisplay'
import { SwapImpactAside } from '@/components/swap/SwapImpactAside'
import { resolveSwapExpectedSlippagePercent } from '@/utils/swapRouteSlippage'
import {
  communityTaxExecuteUsesRouter,
  communityTaxRouteHint,
  extraDebitSellBpsForExecute,
  extraDebitSubmitGate,
  TAX_FUNDING_UNRESOLVED_MESSAGE,
} from '@/utils/taxPreviewMaxSpend'
import { terraBroadcastPendingButtonLabel } from '@/utils/terraBroadcastUi'
import { getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { TRADE_SLIPPAGE_PRESET_CLASS } from '@/utils/tradeMoneyCta'
import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState } from 'react'

interface MarketSimData {
  executionMetadata?: import('@/types').IndexerRouteExecutionMetadata
  executionOperations?: string
  minimumReceive?: string
  return_amount: string
  taxFundingVersion?: 1
  /** Pre-tax wallet sim when `return_amount` is post-buy-split (#615). */
  executeAmountOut?: string
  spread_amount: string
  commission_amount: string
  quoteDisclosure: string
  indexerQuoteKind?: IndexerRouteQuoteKind
  indexerOperations?: SwapOperation[]
  routePreflight?: SwapRoutePreflightSpread
  indexerAmountReconciled?: boolean
  economicExplanation?: string
  routeSlippagePercent?: string
}

function hybridParamsFromBookSplit(
  split: ReturnType<typeof getDirectHybridBookSplit>,
  hybridMaxMakers: number
): HybridSwapParams | undefined {
  if (!split?.willSubmitHybrid) return undefined
  return {
    pool_input: split.poolRaw,
    book_input: split.bookRaw,
    max_maker_fills: hybridMaxMakers,
    book_start_hint: null,
  }
}

/**
 * `/trade` Market ticket — mirrors Swap default quoting (GitLab #501):
 * - Hybrid on + empty manual book → `GET /route/solve` (solver-optimized pool/book split)
 * - Hybrid on + typed book leg → Advanced override via `quoteDirectHybridSwap` (`POST`)
 * - Hybrid off → pool-only pair `simulateSwap`
 * - Direct submit applies `hybridParamsWithSubmitCap` to solver or Advanced hybrid (#249 parity with Swap)
 */
export function TradeMarketOrderPanel({
  pairAddr,
  selectedPair,
  pairs,
  side,
  isPaused,
  dockSubmit = false,
  interactive = true,
  onSubmitChromeChange,
  initialAmountHuman = null,
}: {
  pairAddr: string
  selectedPair: PairInfo | undefined
  pairs: PairInfo[]
  side: 'bid' | 'ask'
  isPaused: boolean
  /** When true, money CTA is published to the ticket footer instead of in-flow (GitLab #527). */
  dockSubmit?: boolean
  /** When false, ignore submit / wallet-open (hidden desktop ticket, GitLab #561). */
  interactive?: boolean
  onSubmitChromeChange?: (model: TradeMarketSubmitChromeModel | null) => void
  initialAmountHuman?: string | null
}) {
  useNumberDisplayMode()
  const address = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)
  const wallet = getConnectedWallet()
  const isWalletConnected = !!address && !!wallet
  const queryClient = useQueryClient()
  const { slippageTolerance, setSlippageTolerance, deadlineSeconds, expertMode } = useDexStore()

  const bookLegInputId = useId()
  const maxMakersInputId = useId()

  const [marketAmountHuman, setMarketAmountHuman] = useState(() =>
    initialAmountHuman && isPositiveDecimalAmount(initialAmountHuman) ? initialAmountHuman : ''
  )
  /** Always-on best execution (#596). Typed book leg overrides via POST. */
  const [bookInputHuman, setBookInputHuman] = useState('')
  const [hybridMaxMakers, setHybridMaxMakers] = useState(8)
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const acceptedImpactKeyRef = useRef<string | null>(null)
  const [reviewedImpactKey, setReviewedImpactKey] = useState<string | null>(null)
  const debouncedBookInputHuman = useDebouncedValue(bookInputHuman, SIM_QUOTE_DEBOUNCE_MS)
  const debouncedHybridMaxMakers = useDebouncedValue(hybridMaxMakers, SIM_QUOTE_DEBOUNCE_MS)

  const token0 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[0]) : ''
  const token1 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[1]) : ''
  const fromToken = side === 'bid' ? token1 : token0
  const routerTaxFunding = useRouterTaxFunding({ enabled: false })
  const taxSell = useCommunityTaxSellBps(fromToken?.startsWith('terra1') ? fromToken : null, { enabled: false })
  const toToken = side === 'bid' ? token0 : token1
  const taxReceive = useCommunityTaxSellBps(toToken?.startsWith('terra1') ? toToken : null, { enabled: false })
  const payDecimalsState = useAssetDecimals(fromToken, {
    lcdEnabled: false,
  })
  const receiveDecimalsState = useAssetDecimals(toToken, {
    lcdEnabled: false,
  })
  const offerDecimals = payDecimalsState.decimals
  const receiveDecimals = receiveDecimalsState.decimals
  const decimalsResolved = offerDecimals != null && receiveDecimals != null
  const decimalsPending = payDecimalsState.pending || receiveDecimalsState.pending
  const rawInputAmount =
    marketAmountHuman.trim() && offerDecimals != null ? toRawAmount(marketAmountHuman.trim(), offerDecimals) : '0'
  const debouncedMarketAmount = useDebouncedValue(marketAmountHuman, SIM_QUOTE_DEBOUNCE_MS)
  const debouncedRawInputAmount =
    debouncedMarketAmount.trim() && offerDecimals != null
      ? toRawAmount(debouncedMarketAmount.trim(), offerDecimals)
      : '0'

  const escrowBalanceQuery = useLimitOrderEscrowBalance(address, fromToken)
  const nativeUlunaQuery = useNativeUlunaBalance(address)

  const liveSplit = useMemo(
    () =>
      getDirectHybridBookSplit({
        isDirect: true,
        fromToken,
        bookInputHuman,
        rawInputAmount,
        hybridMaxMakers,
        payDecimals: offerDecimals,
      }),
    [fromToken, bookInputHuman, rawInputAmount, hybridMaxMakers, offerDecimals]
  )
  const liveHybrid = hybridParamsFromBookSplit(liveSplit, hybridMaxMakers)

  const bookLegMaxResult = useMemo(() => {
    if (!escrowBalanceQuery.data || rawInputAmount === '0' || offerDecimals == null) {
      return { human: '0', spendableRaw: 0n, cappedByGas: false, reserveUluna: 0n }
    }
    return computeMaxSpendableHumanAmount({
      balanceRaw: escrowBalanceQuery.data,
      decimals: offerDecimals,
      assetIsNativeUluna: false,
      context: 'book_leg',
      payAmountRaw: rawInputAmount,
    })
  }, [escrowBalanceQuery.data, offerDecimals, rawInputAmount])

  const placeEscrowGate = useMemo(
    () =>
      offerDecimals == null
        ? { canPlaceLimit: false, userMessage: null as string | null, tone: 'none' as const }
        : evaluateLimitOrderEscrowPlaceGate(marketAmountHuman, offerDecimals, {
            data: escrowBalanceQuery.data,
            isLoading: escrowBalanceQuery.isLoading,
            isError: escrowBalanceQuery.isError,
          }),
    [
      marketAmountHuman,
      offerDecimals,
      escrowBalanceQuery.data,
      escrowBalanceQuery.isLoading,
      escrowBalanceQuery.isError,
    ]
  )

  const maxSpreadStr = useMemo(() => slippagePercentToMaxSpread(slippageTolerance), [slippageTolerance])
  const quoteTrader = useMemo(() => (address ? { trader: address } : undefined), [address])

  const simQuery = useQuery({
    queryKey: [
      'tradeMarketSim',
      pairAddr,
      side,
      debouncedRawInputAmount,
      offerDecimals,
      receiveDecimals,
      debouncedBookInputHuman,
      debouncedHybridMaxMakers,
      slippageTolerance,
      address,
      taxReceive.buyBps ?? null,
      routerTaxFunding.data,
    ],
    placeholderData: keepPreviousData,
    queryFn: async ({ signal }): Promise<MarketSimData> => {
      if (debouncedRawInputAmount === '0') throw new Error(LOCAL_ROUTE_UNAVAILABLE)
      const split =
        debouncedBookInputHuman && isPositiveDecimalAmount(debouncedBookInputHuman)
          ? getDirectHybridBookSplit({
              isDirect: true,
              fromToken,
              bookInputHuman: debouncedBookInputHuman,
              rawInputAmount: debouncedRawInputAmount,
              hybridMaxMakers: debouncedHybridMaxMakers,
              payDecimals: offerDecimals,
            })
          : null
      if (debouncedBookInputHuman && isPositiveDecimalAmount(debouncedBookInputHuman) && !split?.willSubmitHybrid)
        throw new Error('This book split is unavailable. Check the book amount.')
      const quoted = await quoteCw20ViaRouteSolve({
        fromToken,
        toToken,
        simRaw: debouncedRawInputAmount,
        maxMakerFills: debouncedHybridMaxMakers,
        slippageTolerancePercent: slippageTolerance,
        maxSpreadStr: slippagePercentToMaxSpread(slippageTolerance),
        quoteTrader,
        signal,
        declaredHybrid: split?.willSubmitHybrid
          ? [
              {
                pool_input: split.poolRaw,
                book_input: split.bookRaw,
                max_maker_fills: debouncedHybridMaxMakers,
                book_start_hint: null,
              },
            ]
          : undefined,
      })
      if (!quoted) throw new Error(LOCAL_ROUTE_UNAVAILABLE)
      return { ...quoted, quoteDisclosure: quoteDisclosureForIndexerKind(quoted.indexerQuoteKind) }
    },
    enabled:
      decimalsResolved &&
      !!selectedPair &&
      pairAddr.startsWith('terra1') &&
      fromToken.startsWith('terra1') &&
      toToken.startsWith('terra1') &&
      debouncedRawInputAmount !== '0',
    // Skip interval while fetching so in-flight quotes are not cancel/restarted (#484).
    retry: false,
    refetchOnWindowFocus: false,
    staleTime: localQuoteStaleTime,
    refetchInterval: localQuoteRefetchInterval,
  })

  const simInputIntentCurrent =
    rawInputAmount === debouncedRawInputAmount &&
    bookInputHuman === debouncedBookInputHuman &&
    hybridMaxMakers === debouncedHybridMaxMakers
  const hasSettledSimQuote =
    !!simQuery.data &&
    !simQuery.isPlaceholderData &&
    simInputIntentCurrent &&
    !isExpiredLocalQuote(simQuery.data) &&
    !isIndexerLocalQuoteRejection(simQuery.error)
  const pricingDataPending = useQuotePricingDataPending({
    intentKey: JSON.stringify([
      pairAddr,
      side,
      rawInputAmount,
      bookInputHuman,
      hybridMaxMakers,
      slippageTolerance,
      address,
      offerDecimals,
      receiveDecimals,
    ]),
    currentIntent: rawInputAmount !== '0' && simInputIntentCurrent && !simQuery.isPlaceholderData,
    error: simQuery.error,
    hasValidatedQuote: hasSettledSimQuote && !simQuery.isFetching && !simQuery.isError,
    expired: isExpiredLocalQuote(simQuery.data),
  })

  const solverHybridForGas = useMemo(
    () => hybridFromSingleHopIndexerOps(simQuery.data?.indexerOperations),
    [simQuery.data?.indexerOperations]
  )

  // Hybrid (GET or Advanced override) reserves hybrid gas; prefer settled solver / manual split params.
  // Multi-hop market shares the mixed-hop helper (#679) — no second formula.
  const staticMarketGasMin = useMemo(
    () =>
      estimateTradeMarketNetworkFeeUluna(
        simQuery.data?.indexerOperations,
        liveHybrid ?? solverHybridForGas ?? undefined,
        simQuery.data?.taxFundingVersion,
        !!simQuery.data?.executionMetadata
      ),
    [
      simQuery.data?.indexerOperations,
      simQuery.data?.taxFundingVersion,
      simQuery.data?.executionMetadata,
      liveHybrid,
      solverHybridForGas,
    ]
  )
  const marketSwapFee = useMemo(() => {
    const ops = simQuery.data?.indexerOperations
    const hybridOn = !!(liveHybrid ?? solverHybridForGas)
    if (ops && (ops.length >= 2 || !!simQuery.data?.executionMetadata || simQuery.data?.taxFundingVersion === 1)) {
      return tryEstimateSwapNetworkFee({
        isDirectWrap: false,
        needsWrapInput: false,
        hopCount: ops.length,
        cw20RouterOperations: ops,
        taxFundingVersion: simQuery.data?.taxFundingVersion,
        localExecution: !!simQuery.data?.executionMetadata,
      })
    }
    return tryEstimateSwapNetworkFee({
      isDirectWrap: false,
      needsWrapInput: false,
      hopCount: 1,
      cw20DirectPair: true,
      cw20Hybrid: hybridOn,
      cw20HybridParams: liveHybrid ?? solverHybridForGas,
    })
  }, [
    simQuery.data?.indexerOperations,
    simQuery.data?.taxFundingVersion,
    simQuery.data?.executionMetadata,
    liveHybrid,
    solverHybridForGas,
  ])
  const marketGasMin = simQuery.data?.executionMetadata ? marketSwapFee.feeUluna : staticMarketGasMin

  const placeNativeGasGate = useMemo(
    () =>
      offerDecimals == null
        ? { canPlaceLimit: false, userMessage: null as string | null, tone: 'none' as const }
        : evaluateMarketSwapNativeGasPlaceGate(
            marketAmountHuman,
            offerDecimals,
            {
              data: nativeUlunaQuery.data,
              isLoading: nativeUlunaQuery.isLoading,
              isError: nativeUlunaQuery.isError,
            },
            marketGasMin,
            'hybrid swap'
          ),
    [
      marketAmountHuman,
      offerDecimals,
      nativeUlunaQuery.data,
      nativeUlunaQuery.isLoading,
      nativeUlunaQuery.isError,
      marketGasMin,
    ]
  )

  const combinedOk = !marketSwapFee.coverageError && placeEscrowGate.canPlaceLimit && placeNativeGasGate.canPlaceLimit
  const inlineGate = placeEscrowGate.userMessage ? placeEscrowGate : placeNativeGasGate

  const priceImpactTooHigh = simQuery.data?.routePreflight?.anyHopExceedsMaxSpread === true
  const expectedSlippagePct = resolveSwapExpectedSlippagePercent(
    simQuery.data?.routeSlippagePercent,
    simQuery.data?.routePreflight?.worstSpreadPercent ?? null
  )

  const theaterRouteQuote = isTheaterRouteQuote(expectedSlippagePct)
  const routeRiskBlocked = swapRouteSlippageBlocksSubmit(expectedSlippagePct, expertMode)

  const hybridSubmitSnapshot = useMemo(
    () => ({
      bookInputHuman: debouncedBookInputHuman,
      hybridMaxMakers: debouncedHybridMaxMakers,
    }),
    [debouncedBookInputHuman, debouncedHybridMaxMakers]
  )

  const { submitPayRaw, simData, minReceived, isSubmitReady, snapshottedHybrid } = useSubmitAlignedSimQuote({
    rawInputAmount,
    debouncedRawInputAmount,
    simQuery: isIndexerLocalQuoteRejection(simQuery.error) ? { ...simQuery, data: undefined } : simQuery,
    slippageTolerance,
    extraSubmitBlocked: pricingDataPending || priceImpactTooHigh || !!liveSplit?.bookExceedsPay || routeRiskBlocked,
    hybrid: {
      enabled: true,
      live: { bookInputHuman, hybridMaxMakers },
      snapshotted: hybridSubmitSnapshot,
    },
  })
  const tradeUsesRouter =
    !!simData?.executionMetadata ||
    simData?.taxFundingVersion === 1 ||
    communityTaxExecuteUsesRouter(simData?.indexerOperations?.length)
  const communityTaxHint = communityTaxRouteHint({
    payIsTax: taxSell.isTaxToken,
    receiveIsTax: taxReceive.isTaxToken,
    usesRouter: tradeUsesRouter,
    sellBps: taxSell.sellBps,
  })
  const taxPreviewQuery = useMemo(
    () =>
      address && fromToken.startsWith('terra1')
        ? resolveCommunityTaxPreviewQuery({
            wallet: address,
            payToken: fromToken,
            usesRouter: tradeUsesRouter,
            directPairAddr: selectedPair?.contract_addr,
            routeOps: simData?.indexerOperations,
            pairs,
            maxSpread: maxSpreadStr,
          })
        : null,
    [address, fromToken, tradeUsesRouter, selectedPair?.contract_addr, simData?.indexerOperations, pairs, maxSpreadStr]
  )
  const taxPreview = useCommunityTaxPreviewDebit({
    token: fromToken?.startsWith('terra1') ? fromToken : null,
    amount: rawInputAmount,
    enabled: !tradeUsesRouter && taxSell.isTaxToken && taxSell.sellBps != null && taxSell.sellBps > 0,
    previewQuery: taxPreviewQuery,
  })
  const extraDebitGate = extraDebitSubmitGate({
    balanceUnresolved: escrowBalanceQuery.isError,
    declaredRaw: tryParseBigInt(rawInputAmount),
    balanceRaw: escrowBalanceQuery.data !== undefined ? tryParseBigInt(escrowBalanceQuery.data) : null,
    debitRaw: taxPreview.debitRaw,
    sellBps: extraDebitSellBpsForExecute(taxSell.sellBps, tradeUsesRouter),
    extraDebitUnresolved: taxSell.extraDebitUnresolved || taxPreview.previewUnresolved,
    isNativePay: !fromToken?.startsWith('terra1'),
  })

  const impactReviewKey = JSON.stringify([
    pairAddr,
    side,
    address,
    rawInputAmount,
    bookInputHuman,
    hybridMaxMakers,
    slippageTolerance,
    deadlineSeconds,
    expertMode,
    simData?.executionMetadata,
    simData?.executionOperations,
    simData?.return_amount,
    simData?.routeSlippagePercent,
  ])
  useEffect(() => {
    acceptedImpactKeyRef.current = null
    setReviewedImpactKey(null)
  }, [impactReviewKey])
  const needsImpactConsent = expectedSlippagePct != null && expectedSlippagePct > SWAP_FUNDED_HIGH_IMPACT_PCT
  const showImpactConfirm = needsImpactConsent && reviewedImpactKey === impactReviewKey

  const swapMutation = useTerraBroadcastMutation({
    toastSuccess: 'Market swap submitted.',
    mutationFn: async () => {
      if (!address || !selectedPair) throw new Error('Connect wallet')
      if (offerDecimals == null || receiveDecimals == null) throw new Error('Token decimals unavailable')
      if (!fromToken.startsWith('terra1')) throw new Error('Market swap requires CW20 pay token')
      if (extraDebitGate.blockSubmit) {
        throw new Error(extraDebitGate.insufficientBalance ? 'Insufficient Balance' : TAX_FUNDING_UNRESOLVED_MESSAGE)
      }
      assertSubmitQuotePayRawAligned(rawInputAmount, debouncedRawInputAmount)
      if (snapshottedHybrid) {
        assertSubmitHybridAligned({ bookInputHuman, hybridMaxMakers }, snapshottedHybrid)
      }
      if (!isSubmitReady || simQuery.isError || simQuery.isFetching || simQuery.isPlaceholderData)
        throw new Error(LOCAL_ROUTE_UNAVAILABLE)
      if (needsImpactConsent && acceptedImpactKeyRef.current !== impactReviewKey)
        throw new Error('Review the price difference first.')
      acceptedImpactKeyRef.current = null
      if (routeRiskBlocked) throw new Error('Price difference is too high')
      if (!simData) throw new Error('Quote unavailable')
      if (marketSwapFee.coverageError) throw new Error(marketSwapFee.coverageError)
      const payRaw = submitPayRaw
      const submitMinReceived = minReceived
      const escrowGate = evaluateLimitOrderEscrowPlaceGate(marketAmountHuman, offerDecimals, escrowBalanceQuery)
      if (!escrowGate.canPlaceLimit) {
        if (!escrowGate.userMessage) throw new Error('Enter amount')
        throw new Error(escrowGate.userMessage)
      }
      if (payRaw === '0') throw new Error('Enter amount')
      const nativeGate = evaluateMarketSwapNativeGasPlaceGate(
        marketAmountHuman,
        offerDecimals,
        nativeUlunaQuery,
        marketGasMin,
        'hybrid swap'
      )
      if (!nativeGate.canPlaceLimit) {
        if (!nativeGate.userMessage) throw new Error('Insufficient LUNC for gas')
        throw new Error(nativeGate.userMessage)
      }
      const maxSpread = maxSpreadStr
      const idxOps = simData.indexerOperations
      if (!idxOps || !simData.executionMetadata || simData.executionOperations !== JSON.stringify(idxOps)) {
        throw new Error(LOCAL_ROUTE_UNAVAILABLE)
      }
      assertLocalRouteSubmit(
        simData.executionMetadata,
        idxOps,
        payRaw,
        maxSpread,
        address,
        submitMinReceived ?? undefined
      )
      const deadline = Math.floor(Date.now() / 1000) + deadlineSeconds
      return executeMultiHopSwap(
        address,
        fromToken,
        payRaw,
        idxOps,
        maxSpread,
        submitMinReceived ?? undefined,
        undefined,
        deadline,
        simData.executionMetadata
      )
    },
    onSuccess: () => {
      sounds.playSuccess()
      setMarketAmountHuman('')
      setReviewedImpactKey(null)
      queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
      queryClient.invalidateQueries({ queryKey: ['pool'] })
      queryClient.invalidateQueries({ queryKey: ['tradeMarketSim'] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPage'] })
      queryClient.invalidateQueries({ queryKey: ['tradeBestBook'] })
    },
    onError: () => sounds.playError(),
  })

  const indexerHybridExec = useMemo(
    () => getIndexerHybridExecutionSummary(simQuery.data?.indexerQuoteKind),
    [simQuery.data?.indexerQuoteKind]
  )

  /** One execution-aligned route line for the market quote card (GitLab #302 — mirrors /swap swap-route-summary). */
  const marketRouteLine = useMemo(
    () =>
      computeSwapRouteDisplay({
        fromToken,
        toToken,
        isWrapOrUnwrap: false,
        nativeRouteInfo: null,
        indexerOperations: simQuery.data?.indexerOperations,
        clientRoute: null,
        isMultiHop: false,
        isDirect: true,
        displaySymbol: getTokenDisplaySymbol,
      }),
    [fromToken, toToken, simQuery.data?.indexerOperations]
  )

  const marketPairContractAddresses = useMemo(
    () =>
      resolveSwapRoutePairAddresses({
        routeOps: simQuery.data?.indexerOperations,
        pairs,
        directPair: selectedPair,
        fromToken,
        toToken,
      }),
    [simQuery.data?.indexerOperations, pairs, selectedPair, fromToken, toToken]
  )

  useQuoteErrorRecovery({
    error: simQuery.error,
    intentKey: JSON.stringify([
      pairAddr,
      side,
      rawInputAmount,
      bookInputHuman,
      hybridMaxMakers,
      slippageTolerance,
      address,
      offerDecimals,
      receiveDecimals,
    ]),
    currentIntent: simQuery.isError && !simQuery.isFetching && simInputIntentCurrent,
    tokenIn: fromToken,
    tokenOut: toToken,
    amount: debouncedRawInputAmount,
    trader: address ?? undefined,
    maxMakerFills: debouncedHybridMaxMakers,
    maxSpread: maxSpreadStr,
    refetch: simQuery.refetch,
  })
  const receiveRaw = simData?.return_amount
  const receiveHuman = theaterRouteQuote
    ? '—'
    : receiveRaw != null && receiveRaw !== '' && receiveDecimals != null
      ? formatTokenAmount(receiveRaw, receiveDecimals, 6)
      : '—'

  const minReceiveHuman = theaterRouteQuote
    ? '—'
    : minReceived != null && minReceived !== '' && receiveDecimals != null
      ? formatTokenAmount(minReceived, receiveDecimals, 6)
      : '—'

  // Same receive stale/loading rules as Swap (#484 keep-previous vs #496 pay change).
  const showReceiveCalculating =
    pricingDataPending ||
    shouldShowSimReceiveCalculating(
      simQuery.isFetching,
      hasSettledSimQuote,
      simQuery.isPlaceholderData,
      rawInputAmount !== debouncedRawInputAmount
    )

  const hasPositivePay = isPositiveDecimalAmount(marketAmountHuman)
  const payAcquireGuidance = useSwapPayAcquireGuidance({
    walletConnected: isWalletConnected,
    address,
    hasPositivePay: hasPositivePay && decimalsResolved,
    hasSettledQuote: hasSettledSimQuote && decimalsResolved,
    payAsset: fromToken,
    paySymbol: getTokenDisplaySymbol(fromToken),
    payDecimals: offerDecimals ?? 6,
    payRaw: tryParseBigInt(rawInputAmount),
    payBalanceRaw:
      isWalletConnected && escrowBalanceQuery.data !== undefined ? tryParseBigInt(escrowBalanceQuery.data) : null,
    expectedSlippagePct,
  })
  const showQuoteOnly = acquireGuidanceShowsQuoteOnly(payAcquireGuidance, hasSettledSimQuote)

  const canSubmit =
    isWalletConnected &&
    !isPaused &&
    decimalsResolved &&
    combinedOk &&
    !swapMutation.isPending &&
    !!selectedPair &&
    rawInputAmount !== '0' &&
    isSubmitReady &&
    !pricingDataPending &&
    !extraDebitGate.blockSubmit &&
    !routeRiskBlocked &&
    !priceImpactTooHigh

  const submitLabel = !isWalletConnected
    ? 'Connect Wallet'
    : showImpactConfirm && canSubmit
      ? 'Confirm market swap'
      : pricingDataPending
        ? 'Wait for price'
        : extraDebitGate.insufficientBalance
          ? 'Insufficient Balance'
          : hasPositivePay && !decimalsResolved
            ? decimalsPending
              ? 'Loading decimals…'
              : 'Token decimals unavailable'
            : terraBroadcastPendingButtonLabel(
                swapMutation.phase,
                swapMutation.isPending,
                priceImpactTooHigh
                  ? 'Hop spread exceeds slippage protection'
                  : routeRiskBlocked
                    ? 'Price difference is too high'
                    : liveSplit?.bookExceedsPay
                      ? 'Book leg exceeds pay'
                      : `Market ${side === 'bid' ? 'buy' : 'sell'}`,
                'Submitting…'
              )

  const swapMutate = swapMutation.mutate
  const swapPhase = swapMutation.phase
  const swapPendingTxHash = swapMutation.pendingTxHash
  const swapIsError = swapMutation.isError
  const swapErrorMessage = swapMutation.isError ? humanizeUserFacingErrorFromUnknown(swapMutation.error) : null
  const swapIsSuccess = swapMutation.isSuccess
  const swapSuccessTxHash = swapMutation.data

  const submitChromeModel = useMemo<TradeMarketSubmitChromeModel>(
    () => ({
      canSubmit,
      disabled: isWalletConnected ? !canSubmit : isPaused,
      label: submitLabel,
      onClick: () => {
        if (!interactive) return
        if (!isWalletConnected) openWalletModal()
        else if (needsImpactConsent && !showImpactConfirm) setReviewedImpactKey(impactReviewKey)
        else {
          acceptedImpactKeyRef.current = impactReviewKey
          swapMutate()
        }
      },
      phase: swapPhase,
      pendingTxHash: swapPendingTxHash,
      isError: swapIsError,
      errorMessage: swapErrorMessage,
      isSuccess: swapIsSuccess,
      successTxHash: swapSuccessTxHash,
    }),
    [
      canSubmit,
      needsImpactConsent,
      showImpactConfirm,
      impactReviewKey,
      isPaused,
      submitLabel,
      interactive,
      isWalletConnected,
      openWalletModal,
      swapMutate,
      swapPhase,
      swapPendingTxHash,
      swapIsError,
      swapErrorMessage,
      swapIsSuccess,
      swapSuccessTxHash,
    ]
  )

  useLayoutEffect(() => {
    if (!dockSubmit) return
    onSubmitChromeChange?.(submitChromeModel)
    return () => onSubmitChromeChange?.(null)
  }, [dockSubmit, onSubmitChromeChange, submitChromeModel])

  if (!selectedPair) return null

  return (
    <LocalQuoteTokenMetadata.Provider value={true}>
      <div className="space-y-3">
        <LimitOrderEscrowAmountField
          compact
          escrowLabel={getTokenDisplaySymbol(fromToken || '—')}
          escrowDecimals={offerDecimals}
          amountHuman={marketAmountHuman}
          onAmountChange={setMarketAmountHuman}
          balanceQuery={escrowBalanceQuery}
          onMax={setMarketAmountHuman}
          walletConnected={isWalletConnected}
          maxContext="market_swap"
          assetIsNativeUluna={fromToken === 'uluna'}
          marketUsesHybrid={true}
          extraDebitSellBps={extraDebitSellBpsForExecute(
            taxSell.sellBps,
            tradeUsesRouter || routerTaxFunding.data === 1
          )}
        />
        {communityTaxHint && (
          <p className="text-[10px]" style={{ color: 'var(--ink-dim)' }} data-testid="trade-sell-tax-extra">
            {communityTaxHint}
          </p>
        )}

        {pricingDataPending && <QuoteDataLoadingProgress testId="trade-market-price-data-loading" />}
        {!pricingDataPending && (simQuery.isLoading || showReceiveCalculating) && rawInputAmount !== '0' && (
          <div
            className="flex items-center gap-2 text-[10px]"
            style={{ color: 'var(--ink-dim)' }}
            role="status"
            aria-live="polite"
            aria-atomic="true"
            data-testid="trade-market-quoting"
          >
            <Spinner />
            Quoting…
          </div>
        )}
        {simQuery.isError &&
          !pricingDataPending &&
          rawInputAmount !== '0' &&
          !showReceiveCalculating &&
          rawInputAmount === debouncedRawInputAmount &&
          !simQuery.isFetching && (
            <p className="text-[10px] alert-error" role="alert" data-testid="trade-market-quote-error">
              {humanizeUserFacingErrorFromUnknown(simQuery.error)}
            </p>
          )}
        {liveSplit?.bookExceedsPay && (
          <p className="text-[10px] alert-error" role="alert" data-testid="trade-market-book-exceeds-pay">
            Book leg cannot exceed pay amount
          </p>
        )}
        {!pricingDataPending && simData && rawInputAmount !== '0' && (
          <div className="flex justify-between gap-2 text-[10px]" data-testid="trade-market-quote">
            <span style={{ color: 'var(--ink-dim)' }}>Expected receive</span>
            <span className="font-mono text-right" data-testid="trade-market-expected-receive">
              {showReceiveCalculating ? (
                <span className="animate-pulse" style={{ color: 'var(--ink-subtle)' }}>
                  Quoting…
                </span>
              ) : (
                <>
                  {receiveHuman} {getTokenDisplaySymbol(toToken)}
                </>
              )}
            </span>
          </div>
        )}
        {showQuoteOnly && simQuery.data && rawInputAmount !== '0' && (
          <p data-testid="trade-market-quote-only" className="text-[10px]" style={{ color: 'var(--ink-subtle)' }}>
            Quote only
          </p>
        )}

        {!pricingDataPending && simData && rawInputAmount !== '0' && !showReceiveCalculating && (
          <SwapImpactAside
            routeSlippagePct={expectedSlippagePct}
            worstHopPercent={simData.routePreflight?.worstSpreadPercent}
            economicExplanation={simData.economicExplanation}
            testId="trade-market-impact-aside"
          />
        )}

        <div data-testid="trade-market-advanced" data-open={advancedOpen ? 'true' : 'false'}>
          <button
            type="button"
            className="cursor-pointer text-[11px] font-semibold uppercase tracking-wide text-left w-full"
            style={{ color: 'var(--cyan)' }}
            data-testid="trade-market-advanced-toggle"
            aria-expanded={advancedOpen}
            onClick={() => setAdvancedOpen((o) => !o)}
          >
            Advanced
          </button>
          {advancedOpen && (
            <div className="mt-2 space-y-2 border-t border-white/10 pt-2">
              <SlippageProtectionPresets
                selectedPercent={slippageTolerance}
                onSelect={(v) => {
                  sounds.playButtonPress()
                  setSlippageTolerance(v)
                }}
                chipClassName={TRADE_SLIPPAGE_PRESET_CLASS}
                groupTestId="trade-market-slippage-presets"
                presetTestIdPrefix="trade-market-slippage-preset-"
                labelClassName="text-[10px]"
                labelStyle={{ color: 'var(--ink-dim)' }}
                showColon
              />
              <div className="space-y-2">
                <p
                  className="text-[10px] leading-snug"
                  style={{ color: 'var(--ink-dim)' }}
                  data-testid="trade-market-hybrid-min-return-notice"
                >
                  Quotes always use the indexer solver for a price-optimal split. Type a book leg only to override. Min
                  return applies to the combined payout.
                </p>
                <div>
                  <label className="label-glass text-[10px]" htmlFor={bookLegInputId}>
                    Book leg override ({getTokenDisplaySymbol(fromToken)})
                  </label>
                  <input
                    id={bookLegInputId}
                    type="text"
                    inputMode="decimal"
                    className="input-glass !text-xs w-full font-mono"
                    value={bookInputHuman}
                    onChange={(e) => {
                      const v = e.target.value
                      if (isDecimalAmountDraft(v)) setBookInputHuman(v)
                    }}
                    placeholder="0.0"
                    data-testid="trade-market-book-leg-input"
                  />
                  {isWalletConnected && fromToken.startsWith('terra1') && offerDecimals != null && (
                    <AmountBalanceActions
                      balanceQuery={escrowBalanceQuery}
                      decimals={offerDecimals}
                      walletConnected={isWalletConnected}
                      compact
                      spendableRaw={bookLegMaxResult.spendableRaw}
                      onMax={() => setBookInputHuman(bookLegMaxResult.human)}
                      testIdMax="trade-market-book-leg-max"
                    />
                  )}
                </div>
                <div>
                  <label className="label-glass text-[10px]" htmlFor={maxMakersInputId}>
                    Max makers
                    <span
                      className="ml-1 cursor-help opacity-70"
                      title="Caps resting limits one swap can fill; remainder goes to the pool."
                    >
                      ⓘ
                    </span>
                  </label>
                  <input
                    id={maxMakersInputId}
                    type="number"
                    className="input-glass !text-xs w-full"
                    min={1}
                    max={256}
                    value={hybridMaxMakers}
                    onChange={(e) => setHybridMaxMakers(Number(e.target.value) || 8)}
                    data-testid="trade-market-max-makers"
                  />
                </div>
              </div>
              {simQuery.data && rawInputAmount !== '0' && (
                <div className="space-y-1 text-[10px]" data-testid="trade-market-quote-extras">
                  {!showQuoteOnly && (
                    <div className="flex justify-between gap-2">
                      <span style={{ color: 'var(--ink-dim)' }}>Min. after slippage</span>
                      <span className="font-mono text-right">
                        {showReceiveCalculating ? (
                          <span style={{ color: 'var(--ink-subtle)' }}>—</span>
                        ) : (
                          <>
                            {minReceiveHuman} {getTokenDisplaySymbol(toToken)}
                          </>
                        )}
                      </span>
                    </div>
                  )}
                  <p style={{ color: 'var(--ink-subtle)' }}>{simQuery.data.quoteDisclosure}</p>
                  {simQuery.data.indexerAmountReconciled && (
                    <p
                      data-testid="trade-market-amount-reconciled"
                      style={{ color: 'var(--color-warning, #f59e0b)' }}
                      role="status"
                    >
                      {DIRECT_HYBRID_AMOUNT_RECONCILED_COPY}
                    </p>
                  )}
                  {indexerHybridExec.show && (
                    <div className="pt-1 border-t border-white/10">
                      <p className="font-semibold uppercase tracking-wide text-[10px]">{indexerHybridExec.title}</p>
                      <p
                        style={{
                          color: indexerHybridExec.degraded ? 'var(--color-warning, #f59e0b)' : 'var(--ink-dim)',
                        }}
                      >
                        {indexerHybridExec.line}
                      </p>
                    </div>
                  )}
                  <TerraClassicTxFeeHint estimate={marketSwapFee} compact data-testid="trade-market-network-fee" />
                  {marketRouteLine && (
                    <div
                      data-testid="trade-market-route-summary"
                      className="flex flex-col gap-0.5 sm:flex-row sm:items-start sm:justify-between pt-1 border-t border-white/10"
                      style={{ color: 'var(--ink-dim)' }}
                    >
                      <span className="uppercase text-[10px] tracking-wide font-medium shrink-0">Route</span>
                      <span className="font-mono text-[10px] sm:text-right break-words min-w-0">{marketRouteLine}</span>
                    </div>
                  )}
                </div>
              )}
              {simQuery.data && rawInputAmount !== '0' && fromToken && toToken && !showReceiveCalculating && (
                <SwapPreSubmitSummary
                  actionLabel="Market swap"
                  offerSymbol={getTokenDisplaySymbol(fromToken)}
                  receiveSymbol={getTokenDisplaySymbol(toToken)}
                  offerAmountHuman={offerDecimals != null ? fromRawAmount(rawInputAmount, offerDecimals) : '—'}
                  receiveAmountHuman={
                    !theaterRouteQuote && simData?.return_amount && receiveDecimals != null
                      ? fromRawAmount(simData.return_amount, receiveDecimals)
                      : '—'
                  }
                  maxSpreadPercent={slippageTolerance}
                  minReceiveHuman={
                    !showQuoteOnly && !theaterRouteQuote && minReceived && receiveDecimals != null
                      ? fromRawAmount(minReceived, receiveDecimals)
                      : null
                  }
                  pairContractAddresses={marketPairContractAddresses}
                  chainFullLabel={getNetworkBadgeCopy().fullLabel}
                  data-testid="trade-market-pre-submit-summary"
                />
              )}
            </div>
          )}
        </div>

        {showImpactConfirm && canSubmit && (
          <p className="alert-error text-xs" data-testid="trade-market-impact-confirm">
            High fair-reference deviation. Click again to confirm.
            <button type="button" className="ml-2 underline" onClick={() => setReviewedImpactKey(null)}>
              Cancel
            </button>
          </p>
        )}

        <SwapPayAcquireGuidanceBanner
          guidance={payAcquireGuidance}
          testIdPrefix="trade-market"
          hideReduce={acquireReduceHiddenForBroadcastPhase(swapMutation.phase)}
          onReduce={(human) => setMarketAmountHuman(human)}
        />
        <LimitOrderEscrowPlaceGuardMessage
          gate={
            payAcquireGuidance.kind.startsWith('insufficient') &&
            inlineGate.userMessage === LIMIT_ORDER_ESCROW_MSG_INSUFFICIENT
              ? { ...inlineGate, userMessage: null }
              : inlineGate
          }
          data-testid="trade-market-place-guard"
        />
        {!dockSubmit && <TradeMarketSubmitChrome model={submitChromeModel} />}
      </div>
    </LocalQuoteTokenMetadata.Provider>
  )
}
