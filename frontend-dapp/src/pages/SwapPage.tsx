import { useState, useEffect, useCallback, useMemo, useId, useRef } from 'react'
import { useSearchParams } from 'react-router-dom'
import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'
import {
  assertSubmitHybridAligned,
  assertSubmitQuotePayRawAligned,
  shouldShowSimReceiveCalculating,
  SIM_QUOTE_DEBOUNCE_MS,
  simQuoteRefetchInterval,
} from '@/utils/quoteDebounce'
import { resolveSimQuoteLoadingLabel } from '@/utils/routeSolveProgress'
import { useRouteSolveProgress } from '@/hooks/useRouteSolveProgress'
import { useSubmitAlignedSimQuote } from '@/hooks/useSubmitAlignedSimQuote'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { useWalletStore } from '@/hooks/useWallet'
import { useDexStore } from '@/stores/dex'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { simulateSwap, swap, getPool, reverseSimulateSwap } from '@/services/terraclassic/pair'
import { quoteDirectHybridSwap, DIRECT_HYBRID_AMOUNT_RECONCILED_COPY } from '@/utils/directHybridQuote'
import { quoteCw20ViaRouteSolve } from '@/utils/cw20RouteSolveQuote'
import {
  computeDirectHybridMinReturn,
  enrichSwapOperationsWithHopMinReturns,
  preflightSwapRouteSpread,
  type SwapRoutePreflightSpread,
} from '@/services/terraclassic/swapRoutePreflight'
import { getPairFeeConfig } from '@/services/terraclassic/settings'
import { getTokenBalance } from '@/services/terraclassic/queries'
import {
  findRoute,
  getAllTokens,
  simulateMultiHopSwap,
  executeMultiHopSwap,
  type SwapOperation,
  isDirectWrapUnwrap,
  findRouteWithNativeSupport,
  simulateNativeSwap,
  executeNativeSwap,
  netCw20AfterNativeWrap,
} from '@/services/terraclassic/router'
import {
  compareTokenCatalog,
  defaultRetailSwapTokenPair,
  filterRetailDiscoveryTokens,
  isRetailHiddenTestToken,
  shouldRejectGemBridgeQuote,
} from '@/utils/pairCatalogRank'
import {
  applySwapQueryParams,
  canonicalSwapSearch,
  parseSwapExactField,
  swapSearchEquals,
} from '@/utils/swapQueryParams'
import { ShareLinkButton } from '@/components/ui/ShareLinkButton'
import {
  buildCanonicalSwapShareUrl,
  resolveNavigatorCanShare,
  resolveNavigatorShare,
  swapShareText,
} from '@/utils/sharePageLink'
import { SHARE_LINK_ARIA_SWAP, SHARE_LINK_TITLE_SWAP } from '@/utils/sharePageLinkCopy'
import { useCoarseNarrowViewport } from '@/hooks/useCoarseNarrowViewport'
import { hybridParamsWithSubmitCap } from '@/services/terraclassic/hybridSwapGas'
import { hybridFromSingleHopIndexerOps, swapOpsRequireRouter } from '@/services/terraclassic/swapRouting'
import {
  queryPausedState,
  checkRateLimitExceeded,
  getNativeForWrapped,
  queryWrapMapperConfig,
  wrapMapperFeeBps,
  wrapUnwrapFeeNote,
  wrapTreasuryMatchesEnv,
} from '@/services/terraclassic/wrapMapper'
import { WrapRateLimitStatus } from '@/components/wrap/WrapRateLimitStatus'
import { DOCS_GITLAB_BASE, WRAP_MAPPER_CONTRACT_ADDRESS } from '@/utils/constants'
import { useSwapPayAcquireGuidance } from '@/hooks/useSwapPayAcquireGuidance'
import { SwapPayAcquireGuidanceBanner } from '@/components/swap/SwapPayAcquireGuidanceBanner'
import { SWAP_FUNDED_HIGH_IMPACT_PCT, acquireGuidanceShowsQuoteOnly } from '@/utils/swapPayAcquireGuidance'
import {
  assetInfoLabel,
  tokenAssetInfo,
  isNativeDenom,
  type HybridSwapParams,
  type IndexerRouteSolveResponse,
  type IndexerRouteQuoteKind,
} from '@/types'
import { sounds } from '@/lib/sounds'
import { FeeDisplay, TxResultAlert, TokenSearchSelect, Spinner, RetryError } from '@/components/ui'
import { TerraBroadcastPendingLink } from '@/components/ui/TerraBroadcastPendingLink'
import { terraBroadcastPendingButtonLabel } from '@/utils/terraBroadcastUi'
import { LcdQueryGate } from '@/components/common/LcdQueryGate'
import { TerraClassicTxFeeHint } from '@/components/common/TerraClassicTxFeeHint'
import { MarketDataServiceOutageBanner } from '@/components/common/MarketDataServiceOutageBanner'
import { fetchCW20TokenInfo, getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { formatTokenAmount, getDecimals, toRawAmount } from '@/utils/formatAmount'
import { isPositiveDecimalAmount, tryParseBigInt } from '@/utils/decimalAmountInput'
import { spreadPercentFromRawSim } from '@/utils/rawAmountMath'
import { computeMaxSpendableHumanAmount } from '@/utils/maxSpendableAmount'
import { useCommunityTaxSellBps } from '@/hooks/useCommunityTaxSellBps'
import { withBuyTaxReceiveDisplay } from '@/utils/communityTaxNetOut'
import {
  communityTaxExecuteUsesRouter,
  communityTaxRouteHint,
  extraDebitSellBpsForExecute,
} from '@/utils/taxPreviewMaxSpend'
import { estimateSwapNetworkFee } from '@/services/terraclassic/swapNetworkFee'
import { evaluateSwapNativeGasGate } from '@/utils/swapNativeGasBalanceGate'
import { AmountBalanceActions } from '@/components/common/AmountBalanceActions'
import { getRouteSolve } from '@/services/indexer/client'
import {
  getDirectHybridBookSplit,
  getDirectHybridSettingsExecutionSummary,
  getIndexerHybridExecutionSummary,
} from '@/utils/swapDisclosure'
import {
  computeSwapRouteDisplay,
  deriveSwapSubmitRouteOps,
  deriveSwapSubmitRouteSource,
  SWAP_ROUTE_INTERMEDIATE_RECONCILED_COPY,
  SWAP_CLIENT_BFS_FALLBACK_COPY,
} from '@/utils/swapRouteDisplay'
import { resolveSwapRoutePairAddresses } from '@/utils/resolveSwapRoutePairAddresses'
import { humanizeUserFacingError, humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'
import { isIndexerPairNotFoundError, isIndexerUnavailableError } from '@/utils/indexerErrors'
import {
  MARKET_DATA_SERVICE_OUTAGE_TITLE,
  SWAP_MARKET_DATA_OUTAGE_LEAD,
  WRAP_CONFIG_UNAVAILABLE_CTA,
  WRAP_RATE_LIMIT_EXCEEDED_MESSAGE,
  WRAP_TREASURY_MISCONFIGURED_CTA,
  WRAP_UNWRAP_EXCHANGE_DEPOSIT_WARNING,
} from '@/utils/marketDataServiceCopy'
import { fetchNativeTransferTaxParams } from '@/utils/nativeTransferTax'
import { applySlippagePercentFloor } from '@/utils/rawAmountMath'
import { detectSwapIndexerOutage } from '@/utils/swapIndexerOutage'
import { FeeDiscountRegistryWarning } from '@/components/feeDiscount/FeeDiscountRegistryWarning'
import { FeeDiscountUnregisteredCta } from '@/components/feeDiscount/FeeDiscountUnregisteredCta'
import { useFeeDiscountRegistryStatus } from '@/hooks/useFeeDiscountRegistryStatus'
import { useQueryManualRetry } from '@/hooks/useQueryManualRetry'
import { useTradingBlacklist } from '@/hooks/useTradingBlacklist'
import { usePairPaused } from '@/hooks/usePairPaused'
import { usePairCodeIdFreeze } from '@/hooks/usePairCodeIdFreeze'
import { PairCodeIdFrozenBanner } from '@/components/common/PairCodeIdFrozenBanner'
import { CODE_ID_FROZEN_CTA } from '@/utils/assetCodeIdFreeze'
import { USER_INCIDENT_FAQ_HREF } from '@/components/legal/legalCopy'
import { ExpertModeModal } from '@/components/swap/ExpertModeModal'
import { SwapAdvancedSettings } from '@/components/swap/SwapAdvancedSettings'
import { SwapPreSubmitSummary } from '@/components/swap/SwapPreSubmitSummary'
import { readSwapSettingsAdvancedOpen, writeSwapSettingsAdvancedOpen } from '@/utils/swapSettingsAdvanced'
import { getNetworkBadgeCopy } from '@/utils/networkDisplay'
import {
  SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT,
  SWAP_EXTREME_SLIPPAGE_WARNING_PCT,
  parseSlippagePercent,
  resolveSwapExpectedSlippagePercent,
  slippageSeverityClass,
} from '@/utils/swapRouteSlippage'
import {
  formatTransactionDeadline,
  HIGH_SLIPPAGE_PROTECTION_WARN_PERCENT,
  isSlippageCustomOutOfRange,
  persistSlippageCustomInput,
  ROUTE_EXECUTION_SLIPPAGE_LABEL,
  ROUTE_EXECUTION_SLIPPAGE_TOOLTIP,
  sanitizeSlippageCustomInput,
  SLIPPAGE_PROTECTION_LABEL,
  TRANSACTION_DEADLINE_LABEL,
} from '@/utils/slippageProtectionCopy'
import { SlippageProtectionPresets } from '@/components/common/SlippageProtectionPresets'
/** Wallet-side simulation result with optional indexer-routing metadata. */
interface SwapSimData {
  return_amount: string
  /** Pre-tax wallet sim when `return_amount` is post-buy-split (#615). */
  executeAmountOut?: string
  spread_amount: string
  commission_amount: string
  /**
   * Post–mapper-fee pre–burn-tax base for router `minimum_receive` on unwrap_output (#512 / R3).
   * When set, submit floor uses this instead of `return_amount` (which is post-tax display).
   */
  routerMinReceiveBase?: string
  /** Indexer best-route cross-rate slippage (GitLab #293). */
  routeSlippagePercent?: string
  spotAmountOut?: string
  indexerQuoteKind?: IndexerRouteQuoteKind
  indexerOperations?: SwapOperation[]
  indexerIntermediateTokens?: string[]
  /** Per-hop pair simulations for router/indexer/native multihop quotes (router sim omits spread). See `docs/swap-max-spread-ux.md` (GitLab #134). */
  routePreflight?: SwapRoutePreflightSpread
  /** Indexer HTTP failed during quote; pool-only LCD fallback may still succeed (GitLab #241). */
  indexerTransportFailed?: boolean
  /** Indexer `intermediate_tokens` disagreed with `router_operations`; display reconciled to ops path (GitLab #450 / SEC-I02 H09). */
  indexerRouteIntermediateReconciled?: boolean
  /** Indexer `estimated_amount_out` disagreed with wallet hybrid sim; display reconciled to wallet (GitLab #471). */
  indexerAmountReconciled?: boolean
}

export default function SwapPage() {
  const address = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)
  const wallet = getConnectedWallet()
  const isWalletConnected = !!address && !!wallet
  const { slippageTolerance, setSlippageTolerance, deadlineSeconds, setDeadlineSeconds, expertMode, setExpertMode } =
    useDexStore()
  const queryClient = useQueryClient()

  const swapCustomSlippagePctInputId = useId()
  const swapCustomDeadlineInputId = useId()
  const routeSlippageTooltipId = useId()
  const swapHybridBookLegAmountInputId = useId()
  const swapHybridMaxMakersInputId = useId()
  const swapYouPayAmountInputId = useId()
  const swapYouReceiveAmountInputId = useId()

  const [inputAmount, setInputAmount] = useState('')
  const [exactField, setExactField] = useState<'input' | 'output'>('input')
  const [fromToken, setFromToken] = useState<string>('')
  const taxSell = useCommunityTaxSellBps(fromToken.startsWith('terra1') ? fromToken : null)
  const [toToken, setToToken] = useState<string>('')
  const taxReceive = useCommunityTaxSellBps(toToken.startsWith('terra1') ? toToken : null)
  const [showSettings, setShowSettings] = useState(false)
  const [showAdvancedSettings, setShowAdvancedSettings] = useState(readSwapSettingsAdvancedOpen)
  const [showExpertModeModal, setShowExpertModeModal] = useState(false)
  const [customSlippage, setCustomSlippage] = useState('')
  const [customDeadlineMinutes, setCustomDeadlineMinutes] = useState('')
  const [showImpactConfirm, setShowImpactConfirm] = useState(false)
  const [indexerRouteResult, setIndexerRouteResult] = useState<IndexerRouteSolveResponse | null>(null)
  const [indexerRouteError, setIndexerRouteError] = useState<string | null>(null)
  const [indexerRouteLoading, setIndexerRouteLoading] = useState(false)
  const [bookInputHuman, setBookInputHuman] = useState('')
  const [hybridMaxMakers, setHybridMaxMakers] = useState(8)
  const debouncedBookInputHuman = useDebouncedValue(bookInputHuman, SIM_QUOTE_DEBOUNCE_MS)
  const debouncedHybridMaxMakers = useDebouncedValue(hybridMaxMakers, SIM_QUOTE_DEBOUNCE_MS)

  const pairsQuery = useQuery({
    queryKey: ['allPairs'],
    queryFn: () => getAllPairsPaginated(),
    staleTime: 60_000,
  })

  const pairs = useMemo(() => pairsQuery.data?.pairs ?? [], [pairsQuery.data])

  /** Full factory + wrap universe (exit-hatch metadata). Picker uses discoveryTokens (#562). */
  const allTokens = useMemo(() => getAllTokens(pairs), [pairs])
  const discoveryTokens = useMemo(
    () => [...filterRetailDiscoveryTokens(allTokens)].sort(compareTokenCatalog),
    [allTokens]
  )

  const [searchParams, setSearchParams] = useSearchParams()
  const swapQueryKey = searchParams.toString()
  const appliedSwapQueryKeyRef = useRef<string | null>(null)
  const coarseNarrowViewport = useCoarseNarrowViewport()

  useEffect(() => {
    // Wait for factory pairs, not wrap-native-only getAllTokens([]). Stamping the
    // apply-once ref against wrap natives drops a listed CW20 until the next
    // search-string change (GitLab #711 AC1).
    if (!pairsQuery.isSuccess) return
    const pair = defaultRetailSwapTokenPair(allTokens)
    if (!pair) return
    const [econFrom, econTo] = pair
    if (appliedSwapQueryKeyRef.current !== swapQueryKey) {
      appliedSwapQueryKeyRef.current = swapQueryKey
      const applied = applySwapQueryParams(searchParams, allTokens, pair)
      setFromToken(applied.payId)
      setToToken(applied.receiveId)
      if (applied.payAmountHuman != null) setInputAmount(applied.payAmountHuman)
      setExactField(parseSwapExactField(searchParams) === 'output' ? 'output' : 'input')
      return
    }
    const fromHidden = !fromToken || isRetailHiddenTestToken(fromToken)
    const toHidden = !toToken || isRetailHiddenTestToken(toToken)
    if (!fromHidden && !toHidden) return
    if (fromHidden) setFromToken(econFrom)
    if (toHidden) setToToken(fromHidden ? econTo : fromToken === econTo ? econFrom : econTo)
  }, [allTokens, fromToken, toToken, searchParams, swapQueryKey, pairsQuery.isSuccess])

  useEffect(() => {
    const cw20Tokens = allTokens.filter((tokenId) => tokenId.startsWith('terra1'))
    cw20Tokens.forEach((tokenId) => {
      void fetchCW20TokenInfo(tokenId)
    })
  }, [allTokens])

  const wrapUnwrapType = fromToken && toToken ? isDirectWrapUnwrap(fromToken, toToken) : null
  const isWrapOrUnwrap = wrapUnwrapType !== null

  const nativeRouteInfo =
    fromToken && toToken && !isWrapOrUnwrap && (isNativeDenom(fromToken) || isNativeDenom(toToken))
      ? findRouteWithNativeSupport(pairs, fromToken, toToken)
      : null

  const route =
    fromToken && toToken && !isWrapOrUnwrap && !nativeRouteInfo ? findRoute(pairs, fromToken, toToken) : null
  const isDirect = route !== null && route.length === 1
  const isMultiHop = route !== null && route.length > 1
  const showHybridBookSubmitWarning = useMemo(() => {
    if (!isDirect || !fromToken.startsWith('terra1')) return false
    const t = bookInputHuman.trim()
    if (!t) return false
    return isPositiveDecimalAmount(t)
  }, [isDirect, fromToken, bookInputHuman])

  /** CW20-only paths may be solvable via indexer hybrid graph even when the local pair list BFS misses (e.g. hop cap). */
  const indexerCw20Eligible =
    !!fromToken &&
    !!toToken &&
    fromToken.startsWith('terra1') &&
    toToken.startsWith('terra1') &&
    !isWrapOrUnwrap &&
    !nativeRouteInfo

  const hasRoute = isWrapOrUnwrap || nativeRouteInfo !== null || route !== null || indexerCw20Eligible

  const checkIndexerRoute = useCallback(async () => {
    if (!fromToken || !toToken) return
    setIndexerRouteError(null)
    setIndexerRouteResult(null)
    if (!fromToken.startsWith('terra1') || !toToken.startsWith('terra1')) {
      setIndexerRouteError(
        humanizeUserFacingError(
          'Indexer route solver only accepts CW20 contract addresses in the asset table. Native-only denoms are not routable via this endpoint.'
        )
      )
      return
    }
    setIndexerRouteLoading(true)
    try {
      const res = await getRouteSolve(fromToken, toToken)
      const hopTokens = [
        res.token_in,
        res.token_out,
        ...(res.intermediate_tokens ?? []),
        ...res.hops.flatMap((h) => [h.offer_token, h.ask_token]),
      ]
      if (shouldRejectGemBridgeQuote(fromToken, toToken, hopTokens)) {
        setIndexerRouteResult(null)
        return
      }
      setIndexerRouteResult(res)
    } catch (e) {
      setIndexerRouteError(humanizeUserFacingErrorFromUnknown(e))
    } finally {
      setIndexerRouteLoading(false)
    }
  }, [fromToken, toToken])

  useEffect(() => {
    setIndexerRouteResult(null)
    setIndexerRouteError(null)
  }, [fromToken, toToken])

  const directPair = pairs.find((p) => {
    const a = assetInfoLabel(p.asset_infos[0])
    const b = assetInfoLabel(p.asset_infos[1])
    return (a === fromToken && b === toToken) || (b === fromToken && a === toToken)
  })

  const offerAssetInfo = fromToken ? tokenAssetInfo(fromToken) : null
  const receiveAssetInfo = toToken ? tokenAssetInfo(toToken) : null

  const poolQuery = useQuery({
    queryKey: ['pool', directPair?.contract_addr],
    queryFn: () => {
      if (!directPair) throw new Error('No pair')
      return getPool(directPair.contract_addr)
    },
    enabled: !!directPair,
    refetchInterval: 15_000,
  })

  const feeQuery = useQuery({
    queryKey: ['feeConfig', directPair?.contract_addr],
    queryFn: () => {
      if (!directPair) throw new Error('No pair')
      return getPairFeeConfig(directPair.contract_addr)
    },
    enabled: !!directPair,
  })

  const {
    discountBps,
    pairDiscountApplies,
    feeDiscountRegistryStatus,
    showFeeDiscountRegistryWarning,
    feeDiscountConfigured,
    discountQuery,
  } = useFeeDiscountRegistryStatus(directPair?.contract_addr)

  const balanceQuery = useQuery({
    queryKey: ['tokenBalance', address, fromToken],
    queryFn: () => {
      if (!address || !offerAssetInfo) throw new Error('Missing params')
      return getTokenBalance(address, offerAssetInfo)
    },
    enabled: !!address && !!offerAssetInfo,
    refetchInterval: 15_000,
  })

  const ulunaBalanceQuery = useQuery({
    queryKey: ['tokenBalance', address, 'uluna'],
    queryFn: () => {
      if (!address) throw new Error('Missing params')
      return getTokenBalance(address, { native_token: { denom: 'uluna' } })
    },
    enabled: !!address,
    refetchInterval: 15_000,
  })

  const offerDecimals = offerAssetInfo ? getDecimals(offerAssetInfo) : 6
  const receiveDecimals = receiveAssetInfo ? getDecimals(receiveAssetInfo) : 6
  const typedPayRaw = inputAmount ? toRawAmount(inputAmount, offerDecimals) : '0'
  const debouncedInputAmount = useDebouncedValue(inputAmount, SIM_QUOTE_DEBOUNCE_MS)
  const debouncedTypedPayRaw = debouncedInputAmount ? toRawAmount(debouncedInputAmount, offerDecimals) : '0'

  useEffect(() => {
    if (!fromToken || !toToken) return
    if (isRetailHiddenTestToken(fromToken) || isRetailHiddenTestToken(toToken)) return
    const fromOk = allTokens.some((t) => t.trim().toLowerCase() === fromToken.trim().toLowerCase())
    const toOk = allTokens.some((t) => t.trim().toLowerCase() === toToken.trim().toLowerCase())
    if (!fromOk || !toOk) return
    const canonical = canonicalSwapSearch({
      payId: fromToken,
      receiveId: toToken,
      amountHuman: debouncedInputAmount,
      exactField: exactField === 'output' ? 'output' : null,
    })
    if (swapSearchEquals(canonical, searchParams)) return
    appliedSwapQueryKeyRef.current = canonical.toString()
    setSearchParams(canonical, { replace: true })
  }, [allTokens, fromToken, toToken, debouncedInputAmount, exactField, searchParams, setSearchParams])

  const canReverseQuote = Boolean(directPair) && isDirect && !isWrapOrUnwrap && !nativeRouteInfo
  const reverseQuoteActive = exactField === 'output' && canReverseQuote && isPositiveDecimalAmount(debouncedInputAmount)
  const reverseAskRaw =
    reverseQuoteActive && receiveAssetInfo ? toRawAmount(debouncedInputAmount, receiveDecimals) : '0'

  const reverseQuery = useQuery({
    queryKey: ['reverseSimulation', directPair?.contract_addr, toToken, reverseAskRaw],
    queryFn: () => reverseSimulateSwap(directPair!.contract_addr, receiveAssetInfo!, reverseAskRaw),
    enabled: reverseQuoteActive && reverseAskRaw !== '0' && !!directPair && !!receiveAssetInfo,
    placeholderData: keepPreviousData,
    refetchInterval: simQuoteRefetchInterval,
  })

  const reverseOfferRaw = reverseQuery.data?.offer_amount ?? null
  const reverseQuoteReady = reverseQuoteActive && !!reverseOfferRaw && reverseOfferRaw !== '0'
  const rawInputAmount = reverseQuoteReady ? reverseOfferRaw : typedPayRaw
  const debouncedRawInputAmount = reverseQuoteReady ? reverseOfferRaw : debouncedTypedPayRaw

  const needsWrapCheck = isWrapOrUnwrap ? wrapUnwrapType === 'wrap' : (nativeRouteInfo?.needsWrapInput ?? false)
  const wrapDenom = needsWrapCheck ? (isNativeDenom(fromToken) ? fromToken : null) : null

  /** Denom for wrap-mapper rate_limit query (wrap input, unwrap output, or wrapped CW20's native). */
  const wrapRateLimitDenom = useMemo(() => {
    if (wrapDenom) return wrapDenom
    if (wrapUnwrapType === 'unwrap' && isNativeDenom(toToken)) return toToken
    if (nativeRouteInfo?.needsWrapInput && isNativeDenom(fromToken)) return fromToken
    if (isNativeDenom(fromToken)) return fromToken
    if (isNativeDenom(toToken)) return toToken
    if (fromToken) {
      const n = getNativeForWrapped(fromToken)
      if (n) return n
    }
    if (toToken) {
      const n = getNativeForWrapped(toToken)
      if (n) return n
    }
    return null
  }, [wrapDenom, wrapUnwrapType, toToken, fromToken, nativeRouteInfo?.needsWrapInput])

  const payIsNativeUluna = isNativeDenom(fromToken)
  /** Hub-typical 2 hops until the client-BFS route is known — do not default Max to 1-hop (#587). */
  const nativeSwapHopCount =
    nativeRouteInfo?.operations?.length ?? (payIsNativeUluna && wrapUnwrapType !== 'wrap' ? 2 : 1)
  const nativeNeedsWrapInput = nativeRouteInfo?.needsWrapInput ?? (payIsNativeUluna && wrapUnwrapType !== 'wrap')
  const nativeNeedsUnwrapOutput =
    nativeRouteInfo?.needsUnwrapOutput ??
    (!!toToken && isNativeDenom(toToken) && wrapUnwrapType !== 'wrap' && wrapUnwrapType !== 'unwrap')
  const payMaxResult = useMemo(() => {
    if (!balanceQuery.data) {
      return { human: '0', spendableRaw: 0n, cappedByGas: false, reserveUluna: 0n }
    }
    return computeMaxSpendableHumanAmount({
      balanceRaw: balanceQuery.data,
      decimals: offerDecimals,
      assetIsNativeUluna: payIsNativeUluna,
      context: payIsNativeUluna ? 'swap_native' : 'swap_cw20',
      nativeSwapHints: payIsNativeUluna
        ? {
            isDirectWrap: wrapUnwrapType === 'wrap',
            needsWrapInput: nativeNeedsWrapInput,
            needsUnwrapOutput: nativeNeedsUnwrapOutput,
            hopCount: nativeSwapHopCount,
          }
        : undefined,
      extraDebitSellBps: extraDebitSellBpsForExecute(taxSell.sellBps, isMultiHop),
    })
  }, [
    balanceQuery.data,
    offerDecimals,
    payIsNativeUluna,
    wrapUnwrapType,
    nativeNeedsWrapInput,
    nativeNeedsUnwrapOutput,
    nativeSwapHopCount,
    taxSell.sellBps,
    isMultiHop,
  ])

  const bookLegMaxResult = useMemo(() => {
    if (!balanceQuery.data || rawInputAmount === '0') {
      return { human: '0', spendableRaw: 0n, cappedByGas: false, reserveUluna: 0n }
    }
    return computeMaxSpendableHumanAmount({
      balanceRaw: balanceQuery.data,
      decimals: offerDecimals,
      assetIsNativeUluna: false,
      context: 'book_leg',
      payAmountRaw: rawInputAmount,
    })
  }, [balanceQuery.data, offerDecimals, rawInputAmount])

  const wrapMapperActive =
    !!WRAP_MAPPER_CONTRACT_ADDRESS &&
    (needsWrapCheck || (isWrapOrUnwrap && wrapUnwrapType === 'unwrap') || (nativeRouteInfo?.needsUnwrapOutput ?? false))

  const showWrapRateLimitStatus =
    !!WRAP_MAPPER_CONTRACT_ADDRESS &&
    !!wrapRateLimitDenom &&
    (isWrapOrUnwrap || wrapMapperActive || !!(nativeRouteInfo?.needsWrapInput || nativeRouteInfo?.needsUnwrapOutput))

  const wrapMapperConfigQuery = useQuery({
    queryKey: ['wrapMapperConfig'],
    queryFn: queryWrapMapperConfig,
    enabled: wrapMapperActive || isWrapOrUnwrap,
    staleTime: 30_000,
    refetchInterval: 30_000,
  })

  const unwrapBurnTaxDenom =
    wrapUnwrapType === 'unwrap' && isNativeDenom(toToken)
      ? toToken
      : nativeRouteInfo?.needsUnwrapOutput && isNativeDenom(toToken)
        ? toToken
        : null

  const unwrapBurnTaxQuery = useQuery({
    queryKey: ['nativeTransferTax', unwrapBurnTaxDenom],
    queryFn: () => fetchNativeTransferTaxParams(unwrapBurnTaxDenom!),
    enabled: !!unwrapBurnTaxDenom,
    staleTime: 60_000,
  })

  const pausedQuery = useQuery({
    queryKey: ['wrapMapperPaused'],
    queryFn: queryPausedState,
    enabled: wrapMapperActive,
    staleTime: 30_000,
  })

  const rateLimitQuery = useQuery({
    queryKey: ['rateLimit', wrapDenom, rawInputAmount],
    queryFn: () => {
      if (!wrapDenom) throw new Error('No denom')
      return checkRateLimitExceeded(wrapDenom, rawInputAmount)
    },
    enabled: !!wrapDenom && !!rawInputAmount && rawInputAmount !== '0',
    staleTime: 15_000,
  })

  const wrapMapperConfig = wrapMapperConfigQuery.data ?? null
  const wrapFeeBps = wrapMapperFeeBps(wrapMapperConfig, 'wrap')
  const unwrapFeeBps = wrapMapperFeeBps(wrapMapperConfig, 'unwrap')
  const wrapNoteFeeBps = wrapUnwrapType === 'unwrap' ? unwrapFeeBps : wrapFeeBps
  const wrapNeedsSafetyGate =
    !!WRAP_MAPPER_CONTRACT_ADDRESS &&
    (wrapMapperActive || isWrapOrUnwrap || !!(nativeRouteInfo?.needsWrapInput || nativeRouteInfo?.needsUnwrapOutput))
  const wrapTreasuryMismatch = !!wrapMapperConfig && !wrapTreasuryMatchesEnv(wrapMapperConfig)
  const wrapConfigUnavailable = wrapNeedsSafetyGate && wrapMapperConfig == null
  const wrapPauseUnknown = wrapMapperActive && pausedQuery.isFetched && pausedQuery.data === null
  const wrapRateLimitUnknown =
    !!wrapDenom &&
    !!rawInputAmount &&
    rawInputAmount !== '0' &&
    rateLimitQuery.isFetched &&
    rateLimitQuery.data === null
  const wrapSafetyUnavailable = wrapConfigUnavailable || wrapPauseUnknown || wrapRateLimitUnknown
  const isWrapPaused = pausedQuery.data === true || wrapMapperConfig?.paused === true
  const isRateLimitExceeded = rateLimitQuery.data === true

  const simQueryKey = useMemo(
    () =>
      [
        'simulation',
        fromToken,
        toToken,
        debouncedRawInputAmount,
        JSON.stringify(route),
        wrapUnwrapType,
        JSON.stringify(nativeRouteInfo),
        debouncedBookInputHuman,
        debouncedHybridMaxMakers,
        slippageTolerance,
        address,
        wrapFeeBps ?? null,
        unwrapFeeBps ?? null,
        wrapMapperConfig != null,
        taxReceive.buyBps ?? null,
      ] as const,
    [
      fromToken,
      toToken,
      debouncedRawInputAmount,
      route,
      wrapUnwrapType,
      nativeRouteInfo,
      debouncedBookInputHuman,
      debouncedHybridMaxMakers,
      slippageTolerance,
      address,
      wrapFeeBps,
      unwrapFeeBps,
      wrapMapperConfig,
      taxReceive.buyBps,
    ]
  )

  const quoteTrader = useMemo(() => (address ? { trader: address } : undefined), [address])

  const simQuery = useQuery({
    queryKey: simQueryKey,
    placeholderData: keepPreviousData,
    queryFn: async ({ signal }): Promise<SwapSimData> => {
      if (!isPositiveDecimalAmount(debouncedInputAmount)) throw new Error('Missing params')

      const simRaw = debouncedRawInputAmount
      const maxSpreadStr = (slippageTolerance / 100).toString()
      let indexerTransportFailed = false

      const noteIndexerFailure = (err: unknown) => {
        if (isIndexerUnavailableError(err)) indexerTransportFailed = true
      }

      const withIndexerOutageFlag = (data: SwapSimData): SwapSimData =>
        indexerTransportFailed ? { ...data, indexerTransportFailed: true } : data

      if (isWrapOrUnwrap) {
        // Fee-aware direct wrap/unwrap preview (#507); unwrap also nets InstantWithdraw burn tax (#512).
        const result = await simulateNativeSwap(simRaw, fromToken, toToken, pairs)
        return {
          return_amount: result.amount,
          spread_amount: '0',
          commission_amount: '0',
          routerMinReceiveBase: result.routerMinReceiveBase,
        }
      }

      if (nativeRouteInfo) {
        const result = await simulateNativeSwap(simRaw, fromToken, toToken, pairs)
        let routePreflight: SwapRoutePreflightSpread | undefined
        if (nativeRouteInfo.operations.length > 0) {
          let preflightOffer = simRaw
          if (nativeRouteInfo.needsWrapInput) {
            preflightOffer = (await netCw20AfterNativeWrap(BigInt(simRaw), fromToken)).toString()
          }
          routePreflight = await preflightSwapRouteSpread(
            nativeRouteInfo.operations,
            preflightOffer,
            maxSpreadStr,
            quoteTrader
          )
        }
        return withIndexerOutageFlag({
          return_amount: result.amount,
          spread_amount: '0',
          commission_amount: '0',
          routerMinReceiveBase: result.routerMinReceiveBase,
          routePreflight,
        })
      }

      // Advanced: manual limit-book split on a direct pair (overrides indexer hybrid for this quote).
      if (isDirect && directPair && fromToken.startsWith('terra1')) {
        const hybridSplit = getDirectHybridBookSplit({
          isDirect: true,
          fromToken,
          bookInputHuman: debouncedBookInputHuman,
          rawInputAmount: simRaw,
          hybridMaxMakers: debouncedHybridMaxMakers,
        })
        if (hybridSplit?.bookExceedsPay) throw new Error('Book leg cannot exceed pay amount')
        if (hybridSplit?.willSubmitHybrid) {
          const hybridParams: HybridSwapParams = {
            pool_input: hybridSplit.poolRaw,
            book_input: hybridSplit.bookRaw,
            max_maker_fills: debouncedHybridMaxMakers,
            book_start_hint: null,
          }
          const quoted = await quoteDirectHybridSwap({
            pairAddress: directPair.contract_addr,
            fromToken,
            toToken,
            offerAssetInfo: tokenAssetInfo(fromToken),
            askAssetInfo: tokenAssetInfo(toToken),
            simRaw,
            hybrid: hybridParams,
            maxSpreadStr,
            quoteTrader,
          })
          return withIndexerOutageFlag({
            ...quoted,
            routeSlippagePercent: quoted.routeSlippagePercent,
            spotAmountOut: quoted.spotAmountOut,
          })
        }
      }

      // Default CW20↔CW20: indexer GET hybrid (always-on best execution, #501 / #596) + wallet sim.
      if (fromToken.startsWith('terra1') && toToken.startsWith('terra1') && simRaw !== '0') {
        try {
          const quoted = await quoteCw20ViaRouteSolve({
            fromToken,
            toToken,
            simRaw,
            maxMakerFills: debouncedHybridMaxMakers,
            slippageTolerancePercent: slippageTolerance,
            maxSpreadStr,
            quoteTrader,
            signal,
          })
          if (quoted) {
            return {
              return_amount: quoted.return_amount,
              executeAmountOut: quoted.executeAmountOut,
              spread_amount: quoted.spread_amount,
              commission_amount: quoted.commission_amount,
              routeSlippagePercent: quoted.routeSlippagePercent,
              spotAmountOut: quoted.spotAmountOut,
              indexerQuoteKind: quoted.indexerQuoteKind,
              indexerOperations: quoted.indexerOperations,
              indexerIntermediateTokens: quoted.indexerIntermediateTokens,
              indexerRouteIntermediateReconciled: quoted.indexerRouteIntermediateReconciled,
              routePreflight: quoted.routePreflight,
            }
          }
        } catch (e) {
          noteIndexerFailure(e)
          /* pool-only fallback */
        }
      }

      if (!route) throw new Error('No route found')
      if (isDirect && directPair) {
        const hybridSplit = getDirectHybridBookSplit({
          isDirect: true,
          fromToken,
          bookInputHuman: debouncedBookInputHuman,
          rawInputAmount: simRaw,
          hybridMaxMakers: debouncedHybridMaxMakers,
        })
        if (hybridSplit?.willSubmitHybrid) {
          const hybridParams: HybridSwapParams = {
            pool_input: hybridSplit.poolRaw,
            book_input: hybridSplit.bookRaw,
            max_maker_fills: debouncedHybridMaxMakers,
            book_start_hint: null,
          }
          const quoted = await quoteDirectHybridSwap({
            pairAddress: directPair.contract_addr,
            fromToken,
            toToken,
            offerAssetInfo: tokenAssetInfo(fromToken),
            askAssetInfo: tokenAssetInfo(toToken),
            simRaw,
            hybrid: hybridParams,
            maxSpreadStr,
            quoteTrader,
          })
          return withIndexerOutageFlag(
            withBuyTaxReceiveDisplay(
              {
                ...quoted,
                routeSlippagePercent: quoted.routeSlippagePercent,
                spotAmountOut: quoted.spotAmountOut,
              },
              taxReceive.buyBps
            )
          )
        }
        const offerInfo = tokenAssetInfo(fromToken)
        const r = await simulateSwap(directPair.contract_addr, offerInfo, simRaw, quoteTrader)
        return withIndexerOutageFlag(withBuyTaxReceiveDisplay(r, taxReceive.buyBps))
      }
      if (isMultiHop && route) {
        const result = await simulateMultiHopSwap(simRaw, route, quoteTrader)
        const routePreflight = await preflightSwapRouteSpread(route, simRaw, maxSpreadStr, quoteTrader)
        return withIndexerOutageFlag({
          return_amount: result.amount,
          spread_amount: '0',
          commission_amount: '0',
          routePreflight,
        })
      }
      throw new Error('No route found')
    },
    enabled: hasRoute && isPositiveDecimalAmount(debouncedInputAmount) && (!reverseQuoteActive || reverseQuoteReady),
    // Skip interval while fetching so slow multi-hop quotes are not cancel/restarted (#484).
    refetchInterval: simQuoteRefetchInterval,
  })

  const simRetry = useQueryManualRetry(simQueryKey, simQuery)

  const routeSolveProgressEnabled =
    !isWrapOrUnwrap &&
    !nativeRouteInfo &&
    fromToken.startsWith('terra1') &&
    toToken.startsWith('terra1') &&
    debouncedRawInputAmount !== '0'

  const {
    progress: routeSolveProgress,
    fetchStartedAtMs,
    nowMs,
  } = useRouteSolveProgress({
    enabled: routeSolveProgressEnabled,
    isFetching: simQuery.isFetching,
    tokenIn: fromToken,
    tokenOut: toToken,
    amountIn: debouncedRawInputAmount,
    trader: quoteTrader?.trader,
    knownDiscountBps: discountQuery.isSuccess ? (discountQuery.data?.discount_bps ?? 0) : undefined,
    maxMakerFills: debouncedHybridMaxMakers,
  })

  // Settled for *current* sim key only — placeholder keepPreviousData is prior-key stale (#496).
  const hasSettledSimQuote = !!simQuery.data && !simQuery.isPlaceholderData
  const payInputsPendingForReceive = rawInputAmount !== debouncedRawInputAmount
  const showSimReceiveCalculating = shouldShowSimReceiveCalculating(
    simQuery.isFetching,
    hasSettledSimQuote,
    simQuery.isPlaceholderData,
    payInputsPendingForReceive
  )

  const simLoadingLabel = resolveSimQuoteLoadingLabel(
    simQuery.isFetching,
    hasSettledSimQuote,
    routeSolveProgress,
    fetchStartedAtMs,
    nowMs,
    'Calculating...'
  )

  const hybridSubmitSnapshot = useMemo(
    () => ({
      bookInputHuman: debouncedBookInputHuman,
      hybridMaxMakers: debouncedHybridMaxMakers,
    }),
    [debouncedBookInputHuman, debouncedHybridMaxMakers]
  )

  /** Book leg stale gate only applies on direct pairs — multi-hop quotes ignore bookInputHuman (#360). */
  const hybridStaleLive = useMemo(
    () => ({
      bookInputHuman: isDirect ? bookInputHuman : debouncedBookInputHuman,
      hybridMaxMakers,
    }),
    [isDirect, bookInputHuman, debouncedBookInputHuman, hybridMaxMakers]
  )

  const submitDirectHybrid = useMemo(() => {
    const split = getDirectHybridBookSplit({
      isDirect,
      fromToken,
      bookInputHuman: debouncedBookInputHuman,
      rawInputAmount: debouncedRawInputAmount,
      hybridMaxMakers: debouncedHybridMaxMakers,
    })
    if (!split?.willSubmitHybrid) return undefined
    return {
      pool_input: split.poolRaw,
      book_input: split.bookRaw,
      max_maker_fills: debouncedHybridMaxMakers,
      book_start_hint: null,
    } satisfies HybridSwapParams
  }, [isDirect, fromToken, debouncedBookInputHuman, debouncedRawInputAmount, debouncedHybridMaxMakers])

  const {
    submitPayRaw,
    simData,
    minReceived,
    isQuoteStale: simQuoteStale,
    snapshottedHybrid,
  } = useSubmitAlignedSimQuote({
    rawInputAmount,
    debouncedRawInputAmount,
    simQuery,
    slippageTolerance,
    hybrid: {
      enabled: true,
      live: hybridStaleLive,
      snapshotted: hybridSubmitSnapshot,
    },
  })
  const communityTaxHint = communityTaxRouteHint({
    payIsTax: taxSell.isTaxToken,
    receiveIsTax: taxReceive.isTaxToken,
    usesRouter: communityTaxExecuteUsesRouter(simData?.indexerOperations?.length, isMultiHop),
    sellBps: taxSell.sellBps,
  })

  const swapBlacklistProbe = useMemo(() => {
    const routeOps = deriveSwapSubmitRouteOps({
      nativeRouteInfo,
      indexerOperations: simData?.indexerOperations,
      clientRoute: route,
    })
    const tokens = new Set<string>()
    if (fromToken?.startsWith('terra1')) tokens.add(fromToken)
    if (toToken?.startsWith('terra1')) tokens.add(toToken)

    if (routeOps && routeOps.length > 0) {
      for (const op of routeOps) {
        const offer = assetInfoLabel(op.terra_swap.offer_asset_info)
        const ask = assetInfoLabel(op.terra_swap.ask_asset_info)
        if (offer.startsWith('terra1')) tokens.add(offer)
        if (ask.startsWith('terra1')) tokens.add(ask)
      }
    }

    const pairAddresses = resolveSwapRoutePairAddresses({
      routeOps,
      pairs,
      directPair,
      fromToken,
      toToken,
    })
    return { tokens: [...tokens], pairAddresses }
  }, [route, nativeRouteInfo, simData?.indexerOperations, directPair, pairs, fromToken, toToken])

  const tradingBlacklist = useTradingBlacklist({
    wallet: address,
    tokens: swapBlacklistProbe.tokens,
    pairAddress: swapBlacklistProbe.pairAddresses.length === 1 ? swapBlacklistProbe.pairAddresses[0] : null,
    pairs: swapBlacklistProbe.pairAddresses.length > 1 ? swapBlacklistProbe.pairAddresses : null,
    enabled: isWalletConnected,
  })

  const pairPaused = usePairPaused({
    pairAddress: swapBlacklistProbe.pairAddresses.length === 1 ? swapBlacklistProbe.pairAddresses[0] : null,
    pairAddresses: swapBlacklistProbe.pairAddresses.length > 1 ? swapBlacklistProbe.pairAddresses : null,
    enabled: swapBlacklistProbe.pairAddresses.length > 0,
  })
  const isPairPaused = pairPaused.isPaused
  const pairCodeIdFreeze = usePairCodeIdFreeze({
    pairAddress: swapBlacklistProbe.pairAddresses.length === 1 ? swapBlacklistProbe.pairAddresses[0] : null,
    pairAddresses: swapBlacklistProbe.pairAddresses.length > 1 ? swapBlacklistProbe.pairAddresses : null,
    enabled: swapBlacklistProbe.pairAddresses.length > 0,
  })
  const isPairCodeIdFrozen = pairCodeIdFreeze.isFrozen

  const swapMutation = useTerraBroadcastMutation({
    toastSuccess: 'Swap submitted.',
    mutationFn: async () => {
      if (!address || !inputAmount) throw new Error('Missing parameters')
      assertSubmitQuotePayRawAligned(rawInputAmount, debouncedRawInputAmount)
      if (snapshottedHybrid) {
        assertSubmitHybridAligned(hybridStaleLive, snapshottedHybrid)
      }
      if (!simData) throw new Error('Quote unavailable')
      const payRaw = submitPayRaw
      // Display min uses post-tax `return_amount`; router R3 checks post-fee pre-tax (#512).
      const submitMinReceived =
        simData.routerMinReceiveBase != null && simData.routerMinReceiveBase !== simData.return_amount
          ? applySlippagePercentFloor(simData.routerMinReceiveBase, slippageTolerance)
          : minReceived
      const maxSpread = (slippageTolerance / 100).toString()

      if (isWrapOrUnwrap || nativeRouteInfo) {
        const deadline = Math.floor(Date.now() / 1000) + deadlineSeconds
        return executeNativeSwap(
          address,
          fromToken,
          toToken,
          payRaw,
          pairs,
          maxSpread,
          submitMinReceived ?? undefined,
          deadline
        )
      }

      const idxOps = simData.indexerOperations
      const deadline = Math.floor(Date.now() / 1000) + deadlineSeconds

      if (swapOpsRequireRouter(idxOps)) {
        const opsForSubmit = await enrichSwapOperationsWithHopMinReturns(
          idxOps!,
          payRaw,
          slippageTolerance,
          quoteTrader
        )
        return executeMultiHopSwap(
          address,
          fromToken,
          payRaw,
          opsForSubmit,
          maxSpread,
          submitMinReceived ?? undefined,
          undefined,
          deadline
        )
      }

      if (!route) throw new Error('No route found')

      if (isDirect && directPair) {
        let hybrid: HybridSwapParams | undefined = hybridFromSingleHopIndexerOps(idxOps)
        if (!hybrid && submitDirectHybrid) {
          hybrid = submitDirectHybrid
        }
        if (hybrid) {
          hybrid = hybridParamsWithSubmitCap(hybrid)
        }
        const directMinReturn =
          hybrid && BigInt(hybrid.book_input) > 0n
            ? await computeDirectHybridMinReturn(
                directPair.contract_addr,
                tokenAssetInfo(fromToken),
                payRaw,
                hybrid,
                slippageTolerance,
                quoteTrader
              )
            : undefined
        return swap(address, fromToken, directPair.contract_addr, payRaw, undefined, maxSpread, undefined, {
          hybrid,
          minReturn: directMinReturn,
          deadline,
        })
      }

      if (isMultiHop && route) {
        const minReceive = submitMinReceived ?? undefined
        const routeForSubmit = await enrichSwapOperationsWithHopMinReturns(
          route,
          payRaw,
          slippageTolerance,
          quoteTrader
        )
        return executeMultiHopSwap(address, fromToken, payRaw, routeForSubmit, maxSpread, minReceive)
      }

      throw new Error('No route found')
    },
    onSuccess: () => {
      sounds.playSuccess()
      setInputAmount('')
      queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
      queryClient.invalidateQueries({ queryKey: ['pool'] })
      queryClient.invalidateQueries({ queryKey: ['simulation'] })
    },
    onError: () => {
      sounds.playError()
    },
  })

  const indexerOutage = detectSwapIndexerOutage(simQuery, simData)
  const hasPositiveInputAmount = isPositiveDecimalAmount(inputAmount)
  const simQuoteUnavailable =
    !isWrapOrUnwrap && hasPositiveInputAmount && (simQuery.isError || (!simQuery.isLoading && !simData))
  const showSimRetryError =
    simQuery.isError &&
    !isWrapOrUnwrap &&
    hasPositiveInputAmount &&
    !indexerOutage &&
    !isIndexerPairNotFoundError(simQuery.error)

  const outputAmount = simData?.return_amount ?? ''
  const commissionAmount = simData?.commission_amount ?? ''

  const hopSpreadPercent = simData
    ? simData.routePreflight != null
      ? simData.routePreflight.worstSpreadPercent
      : spreadPercentFromRawSim(simData.return_amount, simData.commission_amount, simData.spread_amount)
    : null

  const expectedSlippagePct = simData
    ? resolveSwapExpectedSlippagePercent(simData.routeSlippagePercent, hopSpreadPercent)
    : null

  const priceImpact = expectedSlippagePct != null ? expectedSlippagePct.toFixed(2) : hopSpreadPercent

  const routeSlippageBlocked =
    expectedSlippagePct != null && expectedSlippagePct > SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT && !expertMode

  const extremeSlippageWarning = expectedSlippagePct != null && expectedSlippagePct >= SWAP_EXTREME_SLIPPAGE_WARNING_PCT

  const directHybridBookSplit = useMemo(
    () =>
      getDirectHybridBookSplit({
        isDirect,
        fromToken,
        bookInputHuman: debouncedBookInputHuman,
        rawInputAmount: debouncedRawInputAmount,
        hybridMaxMakers: debouncedHybridMaxMakers,
      }),
    [isDirect, fromToken, debouncedBookInputHuman, debouncedRawInputAmount, debouncedHybridMaxMakers]
  )

  const indexerHybridExec = useMemo(
    () => getIndexerHybridExecutionSummary(simData?.indexerQuoteKind),
    [simData?.indexerQuoteKind]
  )

  /** Settings hybrid Execution block; silent when hybrid on + empty manual book (#492). */
  const directHybridSettingsExec = useMemo(
    () => getDirectHybridSettingsExecutionSummary(directHybridBookSplit),
    [directHybridBookSplit]
  )

  /** One execution-aligned path for the trade summary (GitLab #158 — never duplicate indexer vs client labels). */
  const swapRouteLine = useMemo(
    () =>
      computeSwapRouteDisplay({
        fromToken,
        toToken,
        isWrapOrUnwrap: !!isWrapOrUnwrap,
        nativeRouteInfo: nativeRouteInfo,
        indexerIntermediateTokens: simData?.indexerIntermediateTokens,
        indexerOperations: simData?.indexerOperations,
        clientRoute: route,
        isMultiHop,
        isDirect,
        displaySymbol: getTokenDisplaySymbol,
      }),
    [
      fromToken,
      toToken,
      isWrapOrUnwrap,
      nativeRouteInfo,
      simData?.indexerIntermediateTokens,
      simData?.indexerOperations,
      route,
      isMultiHop,
      isDirect,
    ]
  )

  const swapSubmitRouteSource = useMemo(
    () =>
      deriveSwapSubmitRouteSource({
        isWrapOrUnwrap: !!isWrapOrUnwrap,
        nativeRouteInfo,
        indexerOperations: simData?.indexerOperations,
        clientRoute: route,
        isDirect,
        isMultiHop,
      }),
    [isWrapOrUnwrap, nativeRouteInfo, simData?.indexerOperations, route, isDirect, isMultiHop]
  )

  const showClientBfsFallbackLabel = swapSubmitRouteSource === 'client_bfs'
  const showRouteIntermediateReconciledLabel = !!simData?.indexerRouteIntermediateReconciled
  const showDirectHybridAmountReconciledLabel = !!simData?.indexerAmountReconciled

  const swapNetworkFeeEstimate = useMemo(() => {
    const cw20HopCount = simData?.indexerOperations?.length ?? route?.length ?? 1
    const cw20RouterOperations =
      !isWrapOrUnwrap && !nativeRouteInfo && (simData?.indexerOperations?.length ?? 0) >= 2
        ? simData?.indexerOperations
        : undefined
    return estimateSwapNetworkFee({
      isDirectWrap: wrapUnwrapType === 'wrap',
      isDirectUnwrap: wrapUnwrapType === 'unwrap',
      needsWrapInput: nativeNeedsWrapInput,
      needsUnwrapOutput: nativeNeedsUnwrapOutput,
      hopCount: nativeRouteInfo?.operations?.length ?? (isWrapOrUnwrap ? 1 : nativeSwapHopCount || cw20HopCount),
      cw20DirectPair: !!isDirect && !nativeRouteInfo && !isWrapOrUnwrap,
      cw20Hybrid: !!isDirect && isPositiveDecimalAmount(bookInputHuman.trim()),
      cw20RouterOperations,
    })
  }, [
    wrapUnwrapType,
    nativeNeedsWrapInput,
    nativeNeedsUnwrapOutput,
    nativeRouteInfo,
    nativeSwapHopCount,
    isWrapOrUnwrap,
    isDirect,
    bookInputHuman,
    simData?.indexerOperations,
    route?.length,
  ])

  const swapGasGate = evaluateSwapNativeGasGate(
    inputAmount,
    offerDecimals,
    payIsNativeUluna,
    rawInputAmount,
    payIsNativeUluna ? balanceQuery : ulunaBalanceQuery,
    swapNetworkFeeEstimate.feeUluna
  )

  const insufficientBalance =
    hasPositiveInputAmount && balanceQuery.data !== undefined && BigInt(rawInputAmount) > BigInt(balanceQuery.data)

  const payAcquireGuidance = useSwapPayAcquireGuidance({
    walletConnected: isWalletConnected,
    address,
    hasPositivePay: hasPositiveInputAmount,
    hasSettledQuote: hasSettledSimQuote,
    payAsset: fromToken,
    paySymbol: getTokenDisplaySymbol(fromToken),
    payDecimals: offerDecimals,
    payRaw: tryParseBigInt(rawInputAmount),
    payBalanceRaw: isWalletConnected && balanceQuery.data !== undefined ? tryParseBigInt(balanceQuery.data) : null,
    expectedSlippagePct,
  })
  const showQuoteOnly = acquireGuidanceShowsQuoteOnly(payAcquireGuidance, hasSettledSimQuote)

  const defaultActionLabel = wrapUnwrapType === 'wrap' ? 'Wrap' : wrapUnwrapType === 'unwrap' ? 'Unwrap' : 'Swap'
  const defaultPendingLabel =
    wrapUnwrapType === 'wrap' ? 'Wrapping…' : wrapUnwrapType === 'unwrap' ? 'Unwrapping…' : 'Swapping…'

  let buttonText = defaultActionLabel
  let buttonDisabled = false
  if (!isWalletConnected) {
    buttonText = 'Connect Wallet'
    buttonDisabled = false
  } else if (!hasRoute) {
    buttonText = 'No Route'
    buttonDisabled = true
  } else if (wrapTreasuryMismatch) {
    buttonText = WRAP_TREASURY_MISCONFIGURED_CTA
    buttonDisabled = true
  } else if (wrapSafetyUnavailable) {
    buttonText = WRAP_CONFIG_UNAVAILABLE_CTA
    buttonDisabled = true
  } else if (isWrapPaused) {
    buttonText = 'Wrapping is Temporarily Paused'
    buttonDisabled = true
  } else if (isPairPaused) {
    buttonText = 'Pair is paused'
    buttonDisabled = true
  } else if (isPairCodeIdFrozen) {
    buttonText = CODE_ID_FROZEN_CTA
    buttonDisabled = true
  } else if (tradingBlacklist.blocked) {
    buttonText = 'Trading restricted'
    buttonDisabled = true
  } else if (!hasPositiveInputAmount) {
    buttonText = 'Enter Amount'
    buttonDisabled = true
  } else if (isRateLimitExceeded) {
    buttonText = 'Rate Limit Exceeded'
    buttonDisabled = true
  } else if (insufficientBalance) {
    buttonText = 'Insufficient Balance'
    buttonDisabled = true
  } else if (hasPositiveInputAmount && !swapGasGate.canSubmit) {
    buttonText = swapGasGate.userMessage ?? 'Need LUNC for network fee'
    buttonDisabled = true
  } else if (simQuoteUnavailable) {
    buttonText = 'Quote unavailable'
    buttonDisabled = true
  } else if (routeSlippageBlocked) {
    buttonText = 'Slippage is too high'
    buttonDisabled = true
  } else if (simData?.routePreflight?.anyHopExceedsMaxSpread) {
    buttonText = 'Hop spread exceeds slippage protection'
    buttonDisabled = true
  } else if (simQuery.isLoading || simQuoteStale) {
    buttonText = simLoadingLabel
    buttonDisabled = true
  } else if (swapMutation.isPending) {
    buttonText = terraBroadcastPendingButtonLabel(swapMutation.phase, true, defaultActionLabel, defaultPendingLabel)
    buttonDisabled = true
  } else if (showImpactConfirm) {
    buttonText = `Confirm ${defaultActionLabel} (${priceImpact}% impact)`
    buttonDisabled = false
  }

  const handleSlippagePreset = useCallback(
    (value: number) => {
      sounds.playButtonPress()
      setSlippageTolerance(value)
      setCustomSlippage('')
    },
    [setSlippageTolerance]
  )

  const handleCustomSlippage = useCallback(
    (value: string) => {
      const sanitized = sanitizeSlippageCustomInput(value)
      setCustomSlippage(sanitized)
      const persist = persistSlippageCustomInput(sanitized)
      if (persist != null) setSlippageTolerance(persist)
    },
    [setSlippageTolerance]
  )

  const customSlippageError = isSlippageCustomOutOfRange(customSlippage)

  const handleToggleSettings = useCallback(() => {
    sounds.playButtonPress()
    setShowSettings((prev) => !prev)
  }, [])

  const swapShareUrl = useMemo(() => {
    if (!fromToken || !toToken) return null
    if (isRetailHiddenTestToken(fromToken) || isRetailHiddenTestToken(toToken)) return null
    if (typeof window === 'undefined') return null
    return buildCanonicalSwapShareUrl({
      origin: window.location.origin,
      payId: fromToken,
      receiveId: toToken,
      amountHuman: debouncedInputAmount,
      exactField: exactField === 'output' ? 'output' : null,
    })
  }, [fromToken, toToken, debouncedInputAmount, exactField])

  const reversePayHuman = reverseQuoteReady && reverseOfferRaw ? formatTokenAmount(reverseOfferRaw, offerDecimals) : ''
  const payInputValue = reverseQuoteActive ? reversePayHuman : inputAmount
  const quotedReceiveHuman =
    simData && outputAmount && receiveAssetInfo ? formatTokenAmount(outputAmount, getDecimals(receiveAssetInfo)) : ''
  const receiveInputValue = reverseQuoteActive ? inputAmount : quotedReceiveHuman
  const showReversePayCalculating = reverseQuoteActive && !reverseQuoteReady && isPositiveDecimalAmount(inputAmount)

  const applyTypedAmount = (value: string, field: 'input' | 'output') => {
    if (value === '' || /^\d*\.?\d*$/.test(value)) {
      setExactField(field)
      setInputAmount(value)
      setShowImpactConfirm(false)
    }
  }

  const handleOpenSettings = useCallback(() => {
    sounds.playButtonPress()
    setShowSettings(true)
  }, [])

  const handleAdvancedSettingsOpenChange = useCallback((open: boolean) => {
    setShowAdvancedSettings(open)
    writeSwapSettingsAdvancedOpen(open)
  }, [])

  const SWAP_DEADLINE_PRESETS_MIN = [5, 10, 20, 30] as const
  const activeDeadlinePresetMin = SWAP_DEADLINE_PRESETS_MIN.find((m) => deadlineSeconds === m * 60)

  const handleDeadlinePreset = useCallback(
    (minutes: number) => {
      sounds.playButtonPress()
      setDeadlineSeconds(minutes * 60)
      setCustomDeadlineMinutes('')
    },
    [setDeadlineSeconds]
  )

  const handleCustomDeadlineMinutes = useCallback(
    (value: string) => {
      const sanitized = value.replace(/[^\d.]/g, '').replace(/(\.\d*)\./g, '$1')
      setCustomDeadlineMinutes(sanitized)
      const parsed = parseFloat(sanitized)
      if (!isNaN(parsed) && parsed >= 0.5 && parsed <= 60) {
        setDeadlineSeconds(Math.round(parsed * 60))
      } else if (!isNaN(parsed) && parsed > 60) {
        setDeadlineSeconds(3600)
      }
    },
    [setDeadlineSeconds]
  )

  const customDeadlineError =
    customDeadlineMinutes !== '' &&
    (isNaN(parseFloat(customDeadlineMinutes)) ||
      parseFloat(customDeadlineMinutes) < 0.5 ||
      parseFloat(customDeadlineMinutes) > 60)

  return (
    <div className="relative max-w-[500px] mx-auto w-full">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-x-3 sm:inset-x-6 top-6 sm:top-8 h-[78%] rounded-[40px] theme-hero-glow blur-3xl"
      />
      <LcdQueryGate
        query={pairsQuery}
        loadingFallback={
          <div className="shell-panel-strong relative z-10 !p-5 sm:!p-6 flex items-center justify-center gap-3 py-24">
            <Spinner />
            <span className="text-sm uppercase tracking-wide font-medium" style={{ color: 'var(--ink-dim)' }}>
              Loading pairs…
            </span>
          </div>
        }
      >
        <div className="shell-panel-strong relative z-10 !p-5 sm:!p-6">
          {/* Header */}
          <div className="flex items-center justify-between gap-2 mb-4 sm:mb-6">
            <h2 className="text-lg font-semibold uppercase tracking-wide font-heading">Swap</h2>
            <div className="flex items-center gap-2">
              {swapShareUrl ? (
                <ShareLinkButton
                  url={swapShareUrl}
                  title={SHARE_LINK_TITLE_SWAP}
                  text={swapShareText(fromToken, toToken)}
                  ariaLabel={SHARE_LINK_ARIA_SWAP}
                  data-testid="swap-share-link"
                  canShare={coarseNarrowViewport ? resolveNavigatorCanShare() : () => false}
                  share={coarseNarrowViewport ? resolveNavigatorShare() : undefined}
                />
              ) : null}
              <button
                onClick={handleToggleSettings}
                className="btn-muted !text-[11px] sm:!text-xs !px-2.5 sm:!px-3 !py-1"
              >
                Settings
              </button>
            </div>
          </div>

          {indexerOutage && (
            <MarketDataServiceOutageBanner
              testId="swap-market-data-outage-banner"
              title={MARKET_DATA_SERVICE_OUTAGE_TITLE}
              lead={SWAP_MARKET_DATA_OUTAGE_LEAD}
              onRetry={simRetry.retry}
            />
          )}

          {showFeeDiscountRegistryWarning && <FeeDiscountRegistryWarning testId="swap-fee-discount-registry-warning" />}

          {showSimRetryError && (
            <RetryError
              data-testid="swap-quote-retry-error"
              message={humanizeUserFacingErrorFromUnknown(simQuery.error)}
              isRetrying={simRetry.isRetrying}
              onRetry={simRetry.retry}
            />
          )}

          {simQuery.isError && indexerOutage && !isWrapOrUnwrap && hasPositiveInputAmount && (
            <p className="alert-error text-xs mb-4" data-testid="swap-quote-unavailable">
              {humanizeUserFacingErrorFromUnknown(simQuery.error)}
            </p>
          )}

          {/* Slippage Settings */}
          {showSettings && (
            <>
              <div id="swap-slippage-settings" className="mb-4 sm:mb-6 card-glass animate-fade-in-up">
                <SlippageProtectionPresets
                  selectedPercent={slippageTolerance}
                  onSelect={handleSlippagePreset}
                  customActive={Boolean(customSlippage)}
                  chipClassName="tab-glass !text-xs !px-3 !py-1.5"
                  groupTestId="swap-slippage-presets"
                  presetTestIdPrefix="swap-slippage-preset-"
                  labelClassName="label-glass"
                  customSlot={
                    <div className="relative min-w-[5rem] flex-1" data-testid="swap-slippage-custom">
                      <label htmlFor={swapCustomSlippagePctInputId} className="sr-only">
                        Custom slippage protection (percent)
                      </label>
                      <input
                        id={swapCustomSlippagePctInputId}
                        type="text"
                        value={customSlippage}
                        onChange={(e) => handleCustomSlippage(e.target.value)}
                        placeholder="Custom"
                        className="input-glass !text-xs !py-1.5"
                      />
                      <span
                        className="absolute right-3 top-1/2 -translate-y-1/2 text-xs"
                        style={{ color: 'var(--ink-subtle)' }}
                      >
                        %
                      </span>
                    </div>
                  }
                />
                {customSlippageError && (
                  <p
                    className="mt-2 text-xs font-semibold uppercase tracking-wide"
                    style={{ color: 'var(--color-negative)' }}
                  >
                    Must be between 0.01% and 50%
                  </p>
                )}
                {!customSlippageError && slippageTolerance > HIGH_SLIPPAGE_PROTECTION_WARN_PERCENT && (
                  <p
                    className="mt-2 text-xs font-semibold uppercase tracking-wide"
                    style={{ color: 'var(--color-warning, #f59e0b)' }}
                  >
                    High slippage protection increases front-running risk
                  </p>
                )}
                <div className="mt-4 pt-3 border-t" style={{ borderColor: 'var(--border-subtle)' }}>
                  <p className="label-glass mb-3">{TRANSACTION_DEADLINE_LABEL}</p>
                  <div className="flex flex-wrap gap-2" role="group" aria-label="Transaction deadline preset">
                    {SWAP_DEADLINE_PRESETS_MIN.map((minutes) => (
                      <button
                        key={minutes}
                        type="button"
                        onClick={() => handleDeadlinePreset(minutes)}
                        className={`tab-glass !text-xs !px-3 !py-1.5 ${
                          activeDeadlinePresetMin === minutes && !customDeadlineMinutes
                            ? 'tab-glass-active'
                            : 'tab-glass-inactive'
                        }`}
                      >
                        {minutes}m
                      </button>
                    ))}
                    <div className="relative flex-1 min-w-[5rem]">
                      <label htmlFor={swapCustomDeadlineInputId} className="sr-only">
                        Custom transaction deadline (minutes)
                      </label>
                      <input
                        id={swapCustomDeadlineInputId}
                        type="text"
                        inputMode="decimal"
                        value={customDeadlineMinutes}
                        onChange={(e) => handleCustomDeadlineMinutes(e.target.value)}
                        placeholder="Custom"
                        className="input-glass !text-xs !py-1.5"
                        data-testid="swap-deadline-custom"
                      />
                      <span
                        className="absolute right-3 top-1/2 -translate-y-1/2 text-xs"
                        style={{ color: 'var(--ink-subtle)' }}
                      >
                        min
                      </span>
                    </div>
                  </div>
                  {customDeadlineError && (
                    <p
                      className="mt-2 text-xs font-semibold uppercase tracking-wide"
                      style={{ color: 'var(--color-negative)' }}
                    >
                      Must be between 0.5 and 60 minutes
                    </p>
                  )}
                </div>
                <div className="mt-4 pt-3 border-t" style={{ borderColor: 'var(--border-subtle)' }}>
                  <label
                    className="flex items-center gap-2 text-xs cursor-pointer"
                    title={`When off, swaps with expected slippage above ${SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT}% (vs best-route token prices) are blocked.`}
                  >
                    <input
                      type="checkbox"
                      checked={expertMode}
                      onChange={(e) => {
                        if (e.target.checked) {
                          setShowExpertModeModal(true)
                        } else {
                          setExpertMode(false)
                        }
                      }}
                      data-testid="swap-expert-mode-toggle"
                    />
                    Expert Mode
                  </label>
                </div>
              </div>
              <SwapAdvancedSettings
                open={showAdvancedSettings}
                onOpenChange={handleAdvancedSettingsOpenChange}
                isDirect={isDirect}
                isWrapOrUnwrap={isWrapOrUnwrap}
                directPair={directPair}
                fromToken={fromToken}
                toToken={toToken}
                bookInputHuman={bookInputHuman}
                onBookInputHumanChange={setBookInputHuman}
                hybridMaxMakers={hybridMaxMakers}
                onHybridMaxMakersChange={setHybridMaxMakers}
                bookLegAmountInputId={swapHybridBookLegAmountInputId}
                hybridMaxMakersInputId={swapHybridMaxMakersInputId}
                isWalletConnected={isWalletConnected}
                balanceQuery={balanceQuery}
                offerDecimals={offerDecimals}
                bookLegMaxResult={bookLegMaxResult}
                onCheckIndexerRoute={() => void checkIndexerRoute()}
                indexerRouteLoading={indexerRouteLoading}
                indexerRouteError={indexerRouteError}
                indexerRouteResult={indexerRouteResult}
                clientRouteHopCount={route != null ? route.length : null}
              />
            </>
          )}

          {/* You Pay / swap direction / You Receive — abutting cards, control on seam */}
          <div className="swap-io-stack relative mb-4">
            <div className="card-glass swap-io-card-pay !p-4 sm:!p-5">
              <div className="flex flex-col gap-2 mb-3 sm:flex-row sm:items-start sm:justify-between sm:gap-3">
                <label htmlFor={swapYouPayAmountInputId} className="label-glass !mb-0 sm:pt-1">
                  You Pay
                </label>
                <TokenSearchSelect
                  value={fromToken}
                  tokens={discoveryTokens}
                  excludeToken={toToken}
                  onChange={(tokenId) => {
                    sounds.playButtonPress()
                    setFromToken(tokenId)
                    setShowImpactConfirm(false)
                  }}
                  aria-label="Select token you pay"
                  disabled={allTokens.length === 0}
                />
              </div>
              <input
                id={swapYouPayAmountInputId}
                type="text"
                inputMode="decimal"
                value={showReversePayCalculating ? '' : payInputValue}
                onChange={(e) => applyTypedAmount(e.target.value, 'input')}
                placeholder="0.00"
                className="swap-io-amount-input w-full text-[1.75rem] sm:text-2xl font-medium bg-transparent"
                style={{ color: 'var(--ink)' }}
                data-testid="swap-you-pay-amount"
                aria-busy={showReversePayCalculating || undefined}
              />
              {isWalletConnected && (
                <AmountBalanceActions
                  balanceQuery={balanceQuery}
                  decimals={offerDecimals}
                  walletConnected={isWalletConnected}
                  showBalance={!!offerAssetInfo}
                  spendableRaw={payMaxResult.spendableRaw}
                  onMax={() => {
                    setExactField('input')
                    setInputAmount(payMaxResult.human)
                  }}
                  onFraction={(human) => {
                    setExactField('input')
                    setInputAmount(human)
                    setShowImpactConfirm(false)
                  }}
                  testIdMax="swap-pay-max"
                  testIdFractionPrefix="swap-pay-frac"
                />
              )}
              {communityTaxHint && (
                <p className="text-[11px] mt-1" style={{ color: 'var(--ink-dim)' }} data-testid="swap-sell-tax-extra">
                  {communityTaxHint}
                </p>
              )}
            </div>

            <div className="swap-direction-seam relative z-20 flex justify-center pointer-events-none -my-5 sm:-my-[22px]">
              <button
                type="button"
                aria-label="Swap pay and receive tokens"
                onClick={() => {
                  sounds.playButtonPress()
                  const tmp = fromToken
                  setFromToken(toToken)
                  setToToken(tmp)
                  setShowImpactConfirm(false)
                }}
                className="pointer-events-auto swap-direction-btn w-10 h-10 sm:w-11 sm:h-11 rounded-2xl flex items-center justify-center transition-colors"
              >
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden>
                  <path
                    d="M8 1v14M8 1L4 5M8 1l4 4M8 15l-4-4M8 15l4-4"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                </svg>
              </button>
            </div>

            <div className="card-glass swap-io-card-receive !px-4 !pt-6 !pb-4 sm:!px-5 sm:!pt-7 sm:!pb-5">
              <div className="flex flex-col gap-2 mb-3 sm:flex-row sm:items-start sm:justify-between sm:gap-3">
                {canReverseQuote ? (
                  <label htmlFor={swapYouReceiveAmountInputId} className="label-glass !mb-0 sm:pt-1">
                    You Receive
                  </label>
                ) : (
                  <span className="label-glass !mb-0 sm:pt-1">You Receive</span>
                )}
                <TokenSearchSelect
                  value={toToken}
                  tokens={discoveryTokens}
                  excludeToken={fromToken}
                  onChange={(tokenId) => {
                    sounds.playButtonPress()
                    setToToken(tokenId)
                    setShowImpactConfirm(false)
                  }}
                  aria-label="Select token you receive"
                  disabled={allTokens.length === 0}
                />
              </div>
              {canReverseQuote ? (
                <input
                  id={swapYouReceiveAmountInputId}
                  type="text"
                  inputMode="decimal"
                  value={reverseQuoteActive ? inputAmount : showSimReceiveCalculating ? '' : receiveInputValue}
                  onChange={(e) => applyTypedAmount(e.target.value, 'output')}
                  placeholder="0.00"
                  className="swap-io-amount-input w-full text-[1.75rem] sm:text-2xl font-medium bg-transparent"
                  style={{ color: 'var(--ink)' }}
                  data-testid="swap-you-receive"
                  aria-busy={showSimReceiveCalculating && !reverseQuoteActive ? true : undefined}
                />
              ) : (
                <div
                  className="text-[1.75rem] sm:text-2xl font-medium"
                  style={{ color: 'var(--ink)' }}
                  data-testid="swap-you-receive"
                >
                  {showSimReceiveCalculating ? (
                    <span className="animate-pulse" style={{ color: 'var(--ink-subtle)' }} aria-hidden="true">
                      {simLoadingLabel}
                    </span>
                  ) : quotedReceiveHuman ? (
                    quotedReceiveHuman
                  ) : (
                    <span style={{ color: 'var(--ink-subtle)' }}>0.00</span>
                  )}
                </div>
              )}
              {showQuoteOnly && (
                <p className="text-xs mt-1" style={{ color: 'var(--ink-subtle)' }} data-testid="swap-quote-only">
                  Quote only
                </p>
              )}
              {showSimReceiveCalculating && (
                <span className="sr-only" role="status" aria-live="polite" aria-atomic="true">
                  {simLoadingLabel}
                </span>
              )}
            </div>
          </div>

          <div className="mb-4 space-y-2">
            {simData && (indexerHybridExec.show || directHybridSettingsExec.show) && (
              <div
                data-testid="swap-execution-summary"
                className="card-glass text-[11px] sm:text-xs leading-relaxed space-y-2"
                style={{ color: 'var(--ink-dim)' }}
              >
                {indexerHybridExec.show && (
                  <div>
                    <p
                      className="uppercase tracking-wide font-semibold mb-0.5"
                      style={{ color: 'var(--ink-subtle)' }}
                    >{`Execution: ${indexerHybridExec.title}`}</p>
                    <p>{indexerHybridExec.line}</p>
                  </div>
                )}
                {directHybridSettingsExec.show && (
                  <div>
                    <p className="uppercase tracking-wide font-semibold mb-0.5" style={{ color: 'var(--ink-subtle)' }}>
                      {indexerHybridExec.show ? 'Settings — direct pay split' : 'Execution'}
                    </p>
                    {directHybridSettingsExec.variant === 'book_exceeds_pay' && (
                      <p className="font-medium" style={{ color: 'var(--color-negative)' }}>
                        {directHybridSettingsExec.line}
                      </p>
                    )}
                    {directHybridSettingsExec.variant === 'hybrid_manual_split' && (
                      <>
                        <p style={{ color: 'var(--ink)' }}>
                          <span className="font-semibold">Hybrid</span> — pool {directHybridSettingsExec.poolHuman} ·
                          book {directHybridSettingsExec.bookHuman} {getTokenDisplaySymbol(fromToken)}
                        </p>
                        <p className="text-[10px] font-mono" style={{ color: 'var(--ink-subtle)' }}>
                          {directHybridSettingsExec.poolRaw} (pool raw) · {directHybridSettingsExec.bookRaw} (book raw)
                        </p>
                      </>
                    )}
                    {directHybridSettingsExec.variant === 'max_makers_blocked' && (
                      <p style={{ color: 'var(--color-warning, #f59e0b)' }}>{directHybridSettingsExec.line}</p>
                    )}
                  </div>
                )}
                {showDirectHybridAmountReconciledLabel && (
                  <p
                    data-testid="swap-direct-hybrid-amount-reconciled"
                    className="text-[10px] font-sans leading-snug"
                    style={{ color: 'var(--color-warning, #f59e0b)' }}
                    role="status"
                  >
                    {DIRECT_HYBRID_AMOUNT_RECONCILED_COPY}
                  </p>
                )}
              </div>
            )}
            {fromToken && toToken && !hasRoute && (
              <div className="alert-error !text-xs">No route found between these tokens</div>
            )}
          </div>

          {/* Trade Details — default: Route + Min Received; rest collapsed */}
          {simData && (
            <div className="card-glass mb-4 text-xs sm:text-sm space-y-2">
              {swapRouteLine && (
                <div
                  data-testid="swap-route-summary"
                  className="min-w-0 flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between"
                  style={{ color: 'var(--ink-dim)' }}
                >
                  <span className="uppercase text-xs tracking-wide font-medium shrink-0">Route</span>
                  <span className="font-mono text-xs sm:text-right break-words min-w-0">{swapRouteLine}</span>
                </div>
              )}
              {minReceived !== null && !showQuoteOnly && (
                <div
                  className="min-w-0 flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between"
                  style={{ color: 'var(--ink-dim)' }}
                >
                  <span className="uppercase text-xs tracking-wide font-medium">Min Received</span>
                  <span className="font-mono text-xs sm:text-right break-all">
                    {receiveAssetInfo ? formatTokenAmount(minReceived!, getDecimals(receiveAssetInfo)) : minReceived}
                  </span>
                </div>
              )}
              <div
                className="min-w-0 flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between"
                style={{ color: 'var(--ink-dim)' }}
              >
                <TerraClassicTxFeeHint
                  estimate={swapNetworkFeeEstimate}
                  compact
                  data-testid="swap-network-fee"
                  className="m-0"
                />
              </div>
              {priceImpact !== null && (
                <>
                  {extremeSlippageWarning && (
                    <div className="alert-error !text-xs" data-testid="swap-extreme-slippage-warning" role="alert">
                      <p className="font-semibold mb-1">Extreme slippage ({priceImpact}%)</p>
                      <p>
                        This quote is far from the fair cross-rate. It may exploit mispriced pools or show a
                        misleadingly good multi-hop path. Proceed only if you understand the risk.
                      </p>
                    </div>
                  )}
                </>
              )}
              <details data-testid="swap-trade-details">
                <summary
                  className="cursor-pointer uppercase text-xs tracking-wide font-medium select-none"
                  style={{ color: 'var(--ink-subtle)' }}
                >
                  Trade details
                </summary>
                <div className="mt-2 grid grid-cols-2 gap-x-3 gap-y-2 sm:block sm:space-y-2">
                  {poolQuery.data && (
                    <div
                      className="min-w-0 flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between"
                      style={{ color: 'var(--ink-dim)' }}
                    >
                      <span className="uppercase text-xs tracking-wide font-medium">Pool Reserves</span>
                      <span className="font-mono text-xs sm:text-right break-all">
                        {formatTokenAmount(poolQuery.data.assets[0].amount, getDecimals(poolQuery.data.assets[0].info))}{' '}
                        /{' '}
                        {formatTokenAmount(poolQuery.data.assets[1].amount, getDecimals(poolQuery.data.assets[1].info))}
                      </span>
                    </div>
                  )}
                  {feeQuery.data && (
                    <div
                      className="min-w-0 flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between"
                      style={{ color: 'var(--ink-dim)' }}
                      data-testid="swap-fee-row"
                    >
                      <span className="uppercase text-xs tracking-wide font-medium">Fee</span>
                      <FeeDisplay
                        feeBps={feeQuery.data.fee_bps}
                        discountBps={discountBps}
                        commissionAmount={
                          commissionAmount && receiveAssetInfo
                            ? formatTokenAmount(commissionAmount, getDecimals(receiveAssetInfo))
                            : undefined
                        }
                      />
                    </div>
                  )}
                  <TerraClassicTxFeeHint
                    estimate={swapNetworkFeeEstimate}
                    compact
                    showInternals
                    data-testid="swap-network-fee-details"
                    className="m-0 col-span-2"
                  />
                  <p
                    className="col-span-2 text-[10px] leading-snug"
                    style={{ color: 'var(--ink-subtle)' }}
                    data-testid="swap-wallet-fee-note"
                  >
                    If your wallet shows a much larger fee, use this Network fee amount instead.
                  </p>
                  {address &&
                    feeDiscountConfigured &&
                    pairDiscountApplies &&
                    feeDiscountRegistryStatus === 'unregistered' && (
                      <FeeDiscountUnregisteredCta testId="swap-fee-discount-unregistered-cta" className="col-span-2" />
                    )}
                  {swapRouteLine &&
                    (isWrapOrUnwrap ||
                      nativeRouteInfo ||
                      showClientBfsFallbackLabel ||
                      showRouteIntermediateReconciledLabel) && (
                      <div
                        className="col-span-2 space-y-0.5 font-mono text-xs sm:text-right break-words min-w-0"
                        style={{ color: 'var(--ink-dim)' }}
                      >
                        {isWrapOrUnwrap && wrapUnwrapType && (
                          <span
                            className="block text-[10px] font-sans"
                            style={{ color: 'var(--ink-subtle)' }}
                            data-testid="swap-wrap-fee-note"
                          >
                            {wrapUnwrapFeeNote(
                              wrapUnwrapType,
                              wrapNoteFeeBps,
                              wrapUnwrapType === 'unwrap' ? unwrapBurnTaxQuery.data?.rate : null
                            )}
                          </span>
                        )}
                        {wrapUnwrapType === 'unwrap' && (
                          <span
                            className="block text-[10px] font-sans leading-snug"
                            style={{ color: 'var(--color-warning, #f59e0b)' }}
                            data-testid="swap-unwrap-exchange-warning"
                            role="note"
                          >
                            {WRAP_UNWRAP_EXCHANGE_DEPOSIT_WARNING}
                          </span>
                        )}
                        {nativeRouteInfo && (nativeRouteInfo.needsWrapInput || nativeRouteInfo.needsUnwrapOutput) && (
                          <span
                            className="block text-[10px] font-sans leading-snug"
                            style={{ color: 'var(--ink-subtle)' }}
                          >
                            This swap will{' '}
                            {nativeRouteInfo.needsWrapInput && nativeRouteInfo.needsUnwrapOutput
                              ? 'wrap and unwrap'
                              : nativeRouteInfo.needsWrapInput
                                ? 'wrap'
                                : 'unwrap'}{' '}
                            your tokens
                          </span>
                        )}
                        {showClientBfsFallbackLabel && (
                          <span
                            data-testid="swap-route-source-client-fallback"
                            className="block text-[10px] font-sans leading-snug"
                            style={{ color: 'var(--color-warning, #f59e0b)' }}
                          >
                            {SWAP_CLIENT_BFS_FALLBACK_COPY}
                          </span>
                        )}
                        {showRouteIntermediateReconciledLabel && (
                          <span
                            data-testid="swap-route-intermediate-reconciled"
                            className="block text-[10px] font-sans leading-snug"
                            style={{ color: 'var(--ink-subtle)' }}
                            role="status"
                          >
                            {SWAP_ROUTE_INTERMEDIATE_RECONCILED_COPY}
                          </span>
                        )}
                      </div>
                    )}
                  {priceImpact !== null && (
                    <>
                      <div
                        className="min-w-0 flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between"
                        style={{ color: 'var(--ink-dim)' }}
                        data-testid="swap-expected-slippage"
                      >
                        <span className="inline-flex items-center gap-1 uppercase text-xs tracking-wide font-medium">
                          {ROUTE_EXECUTION_SLIPPAGE_LABEL}
                          <span
                            className="inline-flex h-4 w-4 cursor-help select-none items-center justify-center rounded-full border border-white/25 text-[9px] font-bold leading-none normal-case"
                            tabIndex={0}
                            role="img"
                            aria-label="Expected slippage help"
                            aria-describedby={routeSlippageTooltipId}
                            title={ROUTE_EXECUTION_SLIPPAGE_TOOLTIP}
                          >
                            i
                          </span>
                          <span id={routeSlippageTooltipId} className="sr-only">
                            {ROUTE_EXECUTION_SLIPPAGE_TOOLTIP}
                          </span>
                        </span>
                        <span className={slippageSeverityClass(parseSlippagePercent(priceImpact) ?? 0)}>
                          {priceImpact}%
                        </span>
                      </div>
                      {simData?.routeSlippagePercent && (
                        <p className="col-span-2 text-[10px] leading-snug" style={{ color: 'var(--ink-subtle)' }}>
                          Hop spread: {hopSpreadPercent ?? '—'}%.
                        </p>
                      )}
                    </>
                  )}
                  <button
                    type="button"
                    onClick={handleOpenSettings}
                    aria-expanded={showSettings}
                    aria-controls="swap-slippage-settings"
                    className="min-w-0 flex flex-col gap-1 text-left sm:flex-row sm:items-start sm:justify-between"
                    style={{ color: 'var(--ink-dim)' }}
                  >
                    <span className="uppercase text-xs tracking-wide font-medium">{SLIPPAGE_PROTECTION_LABEL}</span>
                    <span className="inline-flex items-center gap-1">
                      <span>{slippageTolerance}%</span>
                      <span aria-hidden="true" className="text-[10px]" style={{ color: 'var(--cyan)' }}>
                        {showSettings ? '▲' : '▼'}
                      </span>
                    </span>
                  </button>
                  <div
                    className="min-w-0 flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between"
                    style={{ color: 'var(--ink-dim)' }}
                    data-testid="swap-deadline-summary"
                  >
                    <span className="uppercase text-xs tracking-wide font-medium">{TRANSACTION_DEADLINE_LABEL}</span>
                    <span>{formatTransactionDeadline(deadlineSeconds)}</span>
                  </div>
                </div>
              </details>
            </div>
          )}

          {simData?.routePreflight && (
            <p className="card-glass mb-3 text-[11px] sm:text-xs" style={{ color: 'var(--ink-dim)' }}>
              Worst hop spread ≈ {simData.routePreflight.worstSpreadPercent}%.{' '}
              <a
                href="https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs/swap-max-spread-ux.md"
                target="_blank"
                rel="noopener noreferrer"
                className="underline"
              >
                Docs
              </a>
            </p>
          )}
          {routeSlippageBlocked && (
            <div className="alert-error mb-3 text-xs" role="alert" data-testid="swap-slippage-blocked">
              <p className="font-semibold mb-1">Slippage is too high</p>
              <p className="mb-2">
                Expected slippage is {priceImpact}% (above {SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT}%). Enable Expert Mode
                to continue.
              </p>
              <p className="text-[10px]" style={{ color: 'var(--ink-subtle)' }}>
                Dangerous: Enable Expert Mode to Swap Anyway:{' '}
                <button
                  type="button"
                  className="underline font-semibold uppercase tracking-wide"
                  style={{ color: 'var(--cyan)' }}
                  onClick={() => setShowExpertModeModal(true)}
                  data-testid="swap-enable-expert-mode"
                >
                  Enable Expert Mode
                </button>
              </p>
            </div>
          )}
          {simData?.routePreflight?.anyHopExceedsMaxSpread && (
            <div className="alert-error mb-3 text-xs" role="alert">
              <p className="font-semibold mb-1">Insufficient liquidity for this trade size</p>
              <p>Route impact exceeds your {slippageTolerance}% protection — try a smaller amount or another route.</p>
            </div>
          )}
          {(showHybridBookSubmitWarning ||
            simData?.indexerQuoteKind === 'indexer_hybrid_lcd' ||
            simData?.indexerQuoteKind === 'indexer_hybrid_lcd_degraded') && (
            <p className="alert-error mb-3 text-xs" role="alert">
              Quote may change before submit.{' '}
              <a
                href={`${DOCS_GITLAB_BASE}/limit-orders.md`}
                target="_blank"
                rel="noopener noreferrer"
                className="underline"
              >
                Docs
              </a>
            </p>
          )}
          {isPairPaused && (
            <div className="alert-error mb-3 text-xs space-y-2" role="alert" data-testid="swap-pair-paused-banner">
              <p>
                Pair paused.{' '}
                <a className="underline" href={USER_INCIDENT_FAQ_HREF} target="_blank" rel="noopener noreferrer">
                  Docs
                </a>
              </p>
            </div>
          )}
          {isPairCodeIdFrozen && <PairCodeIdFrozenBanner testId="swap-pair-code-id-frozen-banner" />}
          {tradingBlacklist.blocked && tradingBlacklist.message && (
            <div className="alert-error mb-3 text-xs" role="alert">
              <p>{tradingBlacklist.message}</p>
            </div>
          )}
          {showWrapRateLimitStatus && wrapRateLimitDenom && (
            <div className="mb-3 shell-panel px-3 py-2" data-testid="swap-wrap-rate-limit-panel">
              <WrapRateLimitStatus
                denom={wrapRateLimitDenom}
                symbol={getTokenDisplaySymbol(wrapRateLimitDenom)}
                enabled={showWrapRateLimitStatus}
                testId="swap-wrap-rate-limit"
              />
            </div>
          )}
          {isRateLimitExceeded && (
            <div className="alert-error mb-3 text-xs" role="alert" data-testid="swap-wrap-rate-limit-banner">
              <p>{WRAP_RATE_LIMIT_EXCEEDED_MESSAGE}</p>
            </div>
          )}
          {simData && hasPositiveInputAmount && fromToken && toToken && (
            <details className="mb-3" data-testid="swap-pre-submit-details">
              <summary
                className="cursor-pointer uppercase text-xs tracking-wide font-medium select-none mb-2"
                style={{ color: 'var(--ink-subtle)' }}
              >
                Signing details
              </summary>
              <SwapPreSubmitSummary
                offerSymbol={getTokenDisplaySymbol(fromToken)}
                receiveSymbol={getTokenDisplaySymbol(toToken)}
                offerAmountHuman={reverseQuoteActive ? reversePayHuman : inputAmount}
                receiveAmountHuman={
                  outputAmount && receiveAssetInfo
                    ? formatTokenAmount(outputAmount, getDecimals(receiveAssetInfo))
                    : '—'
                }
                maxSpreadPercent={slippageTolerance}
                minReceiveHuman={
                  !showQuoteOnly && minReceived != null && receiveAssetInfo
                    ? formatTokenAmount(minReceived, getDecimals(receiveAssetInfo))
                    : null
                }
                pairContractAddresses={swapBlacklistProbe.pairAddresses}
                chainFullLabel={getNetworkBadgeCopy().fullLabel}
              />
            </details>
          )}
          {/* Swap Button */}
          <SwapPayAcquireGuidanceBanner
            guidance={payAcquireGuidance}
            testIdPrefix="swap"
            onReduce={(human) => {
              setExactField('input')
              setInputAmount(human)
              setShowImpactConfirm(false)
            }}
          />
          {showImpactConfirm && !routeSlippageBlocked && isWalletConnected && (
            <div className="alert-error mb-3 text-xs">
              <p className="font-semibold mb-1">High expected slippage warning</p>
              <p>{priceImpact}% expected slippage — click again to confirm.</p>
            </div>
          )}
          <button
            onClick={() => {
              sounds.playButtonPress()
              if (!isWalletConnected) {
                openWalletModal()
                return
              }
              if (
                priceImpact &&
                (parseSlippagePercent(priceImpact) ?? 0) > SWAP_FUNDED_HIGH_IMPACT_PCT &&
                !showImpactConfirm
              ) {
                setShowImpactConfirm(true)
                return
              }
              setShowImpactConfirm(false)
              swapMutation.mutate()
            }}
            disabled={buttonDisabled}
            className={`w-full py-3.5 sm:py-4 font-semibold text-base ${
              buttonDisabled ? 'btn-disabled !w-full !py-3.5 sm:!py-4' : 'btn-primary btn-cta !w-full !py-3.5 sm:!py-4'
            }`}
          >
            {buttonText}
          </button>

          <TerraBroadcastPendingLink
            phase={swapMutation.phase}
            txHash={swapMutation.pendingTxHash}
            className="mt-2 text-[10px] font-mono break-all"
          />

          {swapMutation.isError && (
            <div className="mt-4">
              <TxResultAlert type="error" message={swapMutation.error?.message ?? 'Swap failed'} />
            </div>
          )}

          {swapMutation.isSuccess && (
            <div className="mt-4">
              <TxResultAlert type="success" message="Swap successful!" txHash={swapMutation.data} />
            </div>
          )}
        </div>
      </LcdQueryGate>
      <ExpertModeModal
        isOpen={showExpertModeModal}
        onClose={() => setShowExpertModeModal(false)}
        onEnable={() => {
          setExpertMode(true)
          setShowExpertModeModal(false)
        }}
      />
    </div>
  )
}
