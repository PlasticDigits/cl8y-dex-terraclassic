import { slippagePercentToMaxSpread } from '@/utils/slippageConstraint'
import { quoteSwapReceiveViaSolver } from '@/utils/swapReceiveQuote'
import { AmountBalanceActions } from '@/components/common/AmountBalanceActions'
import { LcdQueryGate } from '@/components/common/LcdQueryGate'
import { MarketDataServiceOutageBanner } from '@/components/common/MarketDataServiceOutageBanner'
import { SlippageProtectionPresets } from '@/components/common/SlippageProtectionPresets'
import { TerraClassicTxFeeHint } from '@/components/common/TerraClassicTxFeeHint'
import { FeeDiscountRegistryWarning } from '@/components/feeDiscount/FeeDiscountRegistryWarning'
import { FeeDiscountUnregisteredCta } from '@/components/feeDiscount/FeeDiscountUnregisteredCta'
import { USER_INCIDENT_FAQ_HREF } from '@/components/legal/legalCopy'
import { ExpertModeModal } from '@/components/swap/ExpertModeModal'
import { SwapAdvancedSettings } from '@/components/swap/SwapAdvancedSettings'
import { firstTradingNotice, isPreSignTradingError } from '@/utils/tradingNotice'
import { SwapImpactAside } from '@/components/swap/SwapImpactAside'
import { SwapPayAcquireGuidanceBanner } from '@/components/swap/SwapPayAcquireGuidanceBanner'
import { SwapPreSubmitSummary } from '@/components/swap/SwapPreSubmitSummary'
import { SwapSharePairLabel } from '@/components/swap/SwapSharePairLabel'
import { FeeDisplay, RetryError, Spinner, TokenSearchSelect, TxResultAlert } from '@/components/ui'
import { ShareLinkButton } from '@/components/ui/ShareLinkButton'
import { TerraBroadcastPendingLink } from '@/components/ui/TerraBroadcastPendingLink'
import { useAssetDecimals } from '@/hooks/useAssetDecimals'
import { useCoarseNarrowViewport } from '@/hooks/useCoarseNarrowViewport'
import { useCommunityTaxPreviewDebit, useCommunityTaxSellBps } from '@/hooks/useCommunityTaxSellBps'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'
import { useFeeDiscountRegistryStatus } from '@/hooks/useFeeDiscountRegistryStatus'
import { usePairCodeIdFreeze } from '@/hooks/usePairCodeIdFreeze'
import { usePairPaused } from '@/hooks/usePairPaused'
import { useQueryManualRetry } from '@/hooks/useQueryManualRetry'
import { useRouterTaxFunding } from '@/hooks/useRouterTaxFunding'
import { useRouteSolveProgress } from '@/hooks/useRouteSolveProgress'
import { useRouteQuotePromotion } from '@/hooks/useRouteQuotePromotion'
import { useSubmitAlignedSimQuote } from '@/hooks/useSubmitAlignedSimQuote'
import { useSwapPayAcquireGuidance } from '@/hooks/useSwapPayAcquireGuidance'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { LocalQuoteTokenMetadata } from '@/hooks/useTokenDisplayInfo'
import { useTradingBlacklist } from '@/hooks/useTradingBlacklist'
import { useWalletStore } from '@/hooks/useWallet'
import { sounds } from '@/lib/sounds'
import { getAllPairsPaged, getPair, getRouteSolve } from '@/services/indexer/client'
import { getPool } from '@/services/terraclassic/pair'
import { getTokenBalance } from '@/services/terraclassic/queries'
import {
  executeLocalNativeRoute,
  executeMultiHopSwap,
  findRoute,
  findRouteWithNativeSupport,
  getAllTokens,
  isDirectWrapUnwrap,
  type SwapOperation,
} from '@/services/terraclassic/router'
import { getPairFeeConfig } from '@/services/terraclassic/settings'
import { tryEstimateSwapNetworkFee } from '@/services/terraclassic/swapNetworkFee'
import { type SwapRoutePreflightSpread } from '@/services/terraclassic/swapRoutePreflight'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import {
  checkRateLimitExceeded,
  queryPausedState,
  queryWrapMapperConfig,
  wrapMapperFeeBps,
  wrapTreasuryMatchesEnv,
  wrapUnwrapFeeNote,
} from '@/services/terraclassic/wrapMapper'
import { useDexStore } from '@/stores/dex'
import {
  assetInfoLabel,
  indexerPairToPairInfo,
  isNativeDenom,
  tokenAssetInfo,
  type IndexerRouteQuoteKind,
  type IndexerRouteSolveResponse,
} from '@/types'
import { CODE_ID_FROZEN_CTA } from '@/utils/assetCodeIdFreeze'
import { resolveCommunityTaxPreviewQuery } from '@/utils/communityTaxPreviewQuery'
import { WRAP_MAPPER_CONTRACT_ADDRESS } from '@/utils/constants'
import { quoteCw20ViaRouteSolve } from '@/utils/cw20RouteSolveQuote'
import { isPositiveDecimalAmount, tryParseBigInt } from '@/utils/decimalAmountInput'
import { DIRECT_HYBRID_AMOUNT_RECONCILED_COPY } from '@/utils/directHybridQuote'
import { formatNum, formatTokenAmount, fromRawAmount, toRawAmount } from '@/utils/formatAmount'
import { humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'
import {
  isIndexerPairNotFoundError,
  isIndexerRouteValidationError,
  isIndexerLocalQuoteRejection,
} from '@/utils/indexerErrors'
import { useQuoteErrorRecovery } from '@/hooks/useQuoteErrorRecovery'
import { useQuotePricingDataPending } from '@/hooks/useQuotePricingDataPending'
import { QuoteDataLoadingProgress } from '@/components/QuoteDataLoadingProgress'
import { isExpiredLocalQuote, localQuoteStaleTime, localQuoteRefetchInterval } from '@/utils/localQuoteFreshness'
import { assertLocalRouteSubmit, LOCAL_ROUTE_UNAVAILABLE } from '@/utils/localRouteQuote'
import {
  MARKET_DATA_SERVICE_OUTAGE_TITLE,
  SWAP_MARKET_DATA_OUTAGE_LEAD,
  WRAP_CONFIG_UNAVAILABLE_CTA,
  WRAP_RATE_LIMIT_EXCEEDED_MESSAGE,
  WRAP_TREASURY_MISCONFIGURED_CTA,
  WRAP_UNWRAP_EXCHANGE_DEPOSIT_WARNING,
} from '@/utils/marketDataServiceCopy'
import { computeMaxSpendableHumanAmount } from '@/utils/maxSpendableAmount'
import { fetchNativeTransferTaxParams } from '@/utils/nativeTransferTax'
import { wrapMappedSolvePair } from '@/utils/nativeWrapRouteSolve'
import { defaultNativeNeedsWrapInput, defaultNativeWrapHopCount, isNativeUlunaDenom } from '@/utils/nativeWrapSwapHints'
import { getNetworkBadgeCopy } from '@/utils/networkDisplay'
import { useNumberDisplayMode } from '@/utils/numberDisplayPreference'
import {
  compareTokenCatalog,
  defaultRetailSwapTokenPair,
  filterRetailDiscoveryTokens,
  isRetailHiddenTestToken,
  shouldRejectGemBridgeQuote,
} from '@/utils/pairCatalogRank'
import {
  assertSubmitHybridAligned,
  assertSubmitQuotePayRawAligned,
  shouldShowSimReceiveCalculating,
  SIM_QUOTE_DEBOUNCE_MS,
} from '@/utils/quoteDebounce'
import { applySlippagePercentFloor, isPositiveRawAmount, spreadPercentFromRawSim } from '@/utils/rawAmountMath'
import { resolveSwapRoutePairAddresses } from '@/utils/resolveSwapRoutePairAddresses'
import { resolveSimQuoteLoadingLabel } from '@/utils/routeSolveProgress'
import {
  buildCanonicalSwapShareUrl,
  resolveNavigatorCanShare,
  resolveNavigatorShare,
  swapShareAriaLabel,
  swapShareText,
} from '@/utils/sharePageLink'
import { SHARE_LINK_TITLE_SWAP } from '@/utils/sharePageLinkCopy'
import {
  formatTransactionDeadline,
  HIGH_SLIPPAGE_PROTECTION_WARN_PERCENT,
  isSlippageCustomOutOfRange,
  persistSlippageCustomInput,
  sanitizeSlippageCustomInput,
  SLIPPAGE_PROTECTION_LABEL,
  TRANSACTION_DEADLINE_LABEL,
} from '@/utils/slippageProtectionCopy'
import { registrySwapDecimals } from '@/utils/swapAssetDecimals'
import {
  getDirectHybridBookSplit,
  getDirectHybridSettingsExecutionSummary,
  getIndexerHybridExecutionSummary,
} from '@/utils/swapDisclosure'
import { detectSwapIndexerOutage } from '@/utils/swapIndexerOutage'
import { evaluateSwapNativeGasGate } from '@/utils/swapNativeGasBalanceGate'
import {
  acquireGuidanceShowsQuoteOnly,
  acquireReduceHiddenForBroadcastPhase,
  SWAP_FUNDED_HIGH_IMPACT_PCT,
} from '@/utils/swapPayAcquireGuidance'
import {
  applySwapQueryParams,
  canonicalSwapSearch,
  parseSwapExactField,
  swapSearchEquals,
} from '@/utils/swapQueryParams'
import { isTheaterRouteQuote, swapRouteSlippageBlocksSubmit } from '@/utils/swapQuoteAmountScale'
import {
  computeSwapRouteDisplay,
  deriveSwapSubmitRouteOps,
  deriveSwapSubmitRouteSource,
  SWAP_CLIENT_BFS_FALLBACK_COPY,
  SWAP_ROUTE_INTERMEDIATE_RECONCILED_COPY,
} from '@/utils/swapRouteDisplay'
import {
  formatTradingPriceDifference,
  resolveSwapExpectedSlippagePercent,
  SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT,
} from '@/utils/swapRouteSlippage'
import { readSwapSettingsAdvancedOpen, writeSwapSettingsAdvancedOpen } from '@/utils/swapSettingsAdvanced'
import {
  communityTaxExecuteUsesRouter,
  communityTaxRouteHint,
  extraDebitSellBpsForExecute,
  extraDebitSubmitGate,
  TAX_FUNDING_UNRESOLVED_MESSAGE,
} from '@/utils/taxPreviewMaxSpend'
import { terraBroadcastPendingButtonLabel } from '@/utils/terraBroadcastUi'
import { getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback, useEffect, useId, useMemo, useRef, useState } from 'react'
import { useSearchParams } from 'react-router-dom'
/** Wallet-side simulation result with optional indexer-routing metadata. */
interface SwapSimData {
  executionBlocker?: string
  executionMetadata?: import('@/types').IndexerRouteExecutionMetadata
  executionOperations?: string
  minimumReceive?: string
  return_amount: string
  taxFundingVersion?: 1
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
  economicExplanation?: string
  routeSlippagePercent?: string
  spotAmountOut?: string
  indexerQuoteKind?: IndexerRouteQuoteKind
  indexerPairs?: import('@/types').PairInfo[]
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
  const [numberDisplayMode, setNumberDisplayMode] = useNumberDisplayMode()

  const swapCustomSlippagePctInputId = useId()
  const swapCustomDeadlineInputId = useId()
  const swapHybridBookLegAmountInputId = useId()
  const swapHybridMaxMakersInputId = useId()
  const swapYouPayAmountInputId = useId()
  const swapYouReceiveAmountInputId = useId()

  const [inputAmount, setInputAmount] = useState('')
  const [exactField, setExactField] = useState<'input' | 'output'>('input')
  const [fromToken, setFromToken] = useState<string>('')
  const routerTaxFunding = useRouterTaxFunding({ enabled: false })
  const taxSell = useCommunityTaxSellBps(fromToken.startsWith('terra1') ? fromToken : null, { enabled: false })
  const [toToken, setToToken] = useState<string>('')
  const taxReceive = useCommunityTaxSellBps(toToken.startsWith('terra1') ? toToken : null, { enabled: false })
  const [showSettings, setShowSettings] = useState(false)
  const [showAdvancedSettings, setShowAdvancedSettings] = useState(readSwapSettingsAdvancedOpen)
  const [showExpertModeModal, setShowExpertModeModal] = useState(false)
  const [customSlippage, setCustomSlippage] = useState('')
  const [customDeadlineMinutes, setCustomDeadlineMinutes] = useState('')
  const [showImpactConfirm, setShowImpactConfirm] = useState(false)
  const impactAcceptedQuoteRef = useRef<string | null>(null)
  const impactReviewedQuoteRef = useRef<string | null>(null)
  const [indexerRouteResult, setIndexerRouteResult] = useState<IndexerRouteSolveResponse | null>(null)
  const [indexerRouteError, setIndexerRouteError] = useState<string | null>(null)
  const [indexerRouteLoading, setIndexerRouteLoading] = useState(false)
  const [bookInputHuman, setBookInputHuman] = useState('')
  const [hybridMaxMakers, setHybridMaxMakers] = useState(8)
  const debouncedBookInputHuman = useDebouncedValue(bookInputHuman, SIM_QUOTE_DEBOUNCE_MS)
  const debouncedHybridMaxMakers = useDebouncedValue(hybridMaxMakers, SIM_QUOTE_DEBOUNCE_MS)

  const pairsQuery = useQuery({
    queryKey: ['allPairs'],
    queryFn: async () => ({ pairs: (await getAllPairsPaged()).map(indexerPairToPairInfo) }),
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
  // Outbound URL replacement may still be pending when a user flips/picks.
  // Read each actual location once so that old search cannot undo that intent.
  const observedSwapQueryKeyRef = useRef<string | null>(null)
  const coarseNarrowViewport = useCoarseNarrowViewport()

  useEffect(() => {
    // Wait for factory pairs, not wrap-native-only getAllTokens([]). Stamping the
    // apply-once ref against wrap natives drops a listed CW20 until the next
    // search-string change (GitLab #711 AC1).
    if (!pairsQuery.isSuccess) return
    // Explicit links remain usable even when product-default identities are
    // absent. Empty defaults never invent a replacement market (#1473).
    const pair = defaultRetailSwapTokenPair(allTokens) ?? ['', '']
    const [econFrom, econTo] = pair
    if (
      (appliedSwapQueryKeyRef.current !== swapQueryKey && observedSwapQueryKeyRef.current !== swapQueryKey) ||
      (!fromToken && !toToken)
    ) {
      const applied = applySwapQueryParams(searchParams, allTokens, pair)
      observedSwapQueryKeyRef.current = swapQueryKey
      if (!applied.payId && !applied.receiveId) return
      appliedSwapQueryKeyRef.current = swapQueryKey
      setFromToken(applied.payId)
      setToToken(applied.receiveId)
      if (applied.payAmountHuman != null) setInputAmount(applied.payAmountHuman)
      setExactField(parseSwapExactField(searchParams) === 'output' ? 'output' : 'input')
      return
    }
    observedSwapQueryKeyRef.current = swapQueryKey
    const fromHidden = !fromToken || isRetailHiddenTestToken(fromToken)
    const toHidden = !toToken || isRetailHiddenTestToken(toToken)
    if (!fromHidden && !toHidden) return
    if (fromHidden && econFrom) setFromToken(toToken === econFrom ? econTo : econFrom)
    if (toHidden && econTo) setToToken(!fromHidden && fromToken === econTo ? econFrom : econTo)
  }, [allTokens, fromToken, toToken, searchParams, swapQueryKey, pairsQuery.isSuccess])

  const wrapUnwrapType = fromToken && toToken ? isDirectWrapUnwrap(fromToken, toToken) : null
  const isWrapOrUnwrap = wrapUnwrapType !== null

  const wrapSolvePair = fromToken && toToken && !isWrapOrUnwrap ? wrapMappedSolvePair(fromToken, toToken) : null

  const nativeRouteInfo =
    fromToken && toToken && !isWrapOrUnwrap && (isNativeDenom(fromToken) || isNativeDenom(toToken))
      ? findRouteWithNativeSupport(pairs, fromToken, toToken)
      : null

  const route =
    fromToken && toToken && !isWrapOrUnwrap && !nativeRouteInfo && !wrapSolvePair
      ? findRoute(pairs, fromToken, toToken)
      : null
  const isDirect = route !== null && route.length === 1
  const isMultiHop = route !== null && route.length > 1

  /** CW20-only paths may be solvable via indexer hybrid graph even when the local pair list BFS misses (e.g. hop cap). */
  const indexerCw20Eligible =
    !!fromToken &&
    !!toToken &&
    fromToken.startsWith('terra1') &&
    toToken.startsWith('terra1') &&
    !isWrapOrUnwrap &&
    !nativeRouteInfo &&
    !wrapSolvePair

  const hasRoute =
    isWrapOrUnwrap || wrapSolvePair !== null || nativeRouteInfo !== null || route !== null || indexerCw20Eligible

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

  const indexerPairQuery = useQuery({
    queryKey: ['indexer-pair', directPair?.contract_addr],
    queryFn: () => getPair(directPair!.contract_addr),
    enabled: !!directPair?.contract_addr,
    staleTime: 60_000,
    retry: false,
  })

  const payDecimalsState = useAssetDecimals(fromToken, {
    lcdEnabled: false,
    indexerPair: indexerPairQuery.data ?? null,
  })
  const receiveDecimalsState = useAssetDecimals(toToken, {
    lcdEnabled: false,
    indexerPair: indexerPairQuery.data ?? null,
  })
  const offerDecimals = payDecimalsState.decimals
  const receiveDecimals = receiveDecimalsState.decimals
  const decimalsResolved = offerDecimals != null && receiveDecimals != null
  const decimalsPending = payDecimalsState.pending || receiveDecimalsState.pending

  const poolQuery = useQuery({
    queryKey: ['pool', directPair?.contract_addr],
    queryFn: () => {
      if (!directPair) throw new Error('No pair')
      return getPool(directPair.contract_addr)
    },
    enabled: false,
    refetchInterval: 15_000,
  })

  const feeQuery = useQuery({
    queryKey: ['feeConfig', directPair?.contract_addr],
    queryFn: () => {
      if (!directPair) throw new Error('No pair')
      return getPairFeeConfig(directPair.contract_addr)
    },
    enabled: false,
  })

  const {
    discountBps,
    pairDiscountApplies,
    feeDiscountRegistryStatus,
    showFeeDiscountRegistryWarning,
    feeDiscountConfigured,
  } = useFeeDiscountRegistryStatus(directPair?.contract_addr, { enabled: false })

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

  const typedPayRaw = inputAmount && offerDecimals != null ? toRawAmount(inputAmount, offerDecimals) : '0'
  const debouncedInputAmount = useDebouncedValue(inputAmount, SIM_QUOTE_DEBOUNCE_MS)
  const debouncedTypedPayRaw =
    debouncedInputAmount && offerDecimals != null ? toRawAmount(debouncedInputAmount, offerDecimals) : '0'

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

  const needsWrapCheck = isWrapOrUnwrap
    ? wrapUnwrapType === 'wrap'
    : nativeRouteInfo?.needsWrapInput || wrapSolvePair?.needsWrapInput || false
  const wrapDenom = needsWrapCheck ? (isNativeDenom(fromToken) ? fromToken : null) : null

  /** Fee-paying native is LUNC only — USTC (`uusd`) wraps but pays gas in uluna (#1264 G1264-4). */
  const payIsNativeUluna = isNativeUlunaDenom(fromToken)
  const payIsNativeDenom = isNativeDenom(fromToken)
  const isDirectWrapOrUnwrap = wrapUnwrapType === 'wrap' || wrapUnwrapType === 'unwrap'
  /** Hub-typical 2 hops until the client-BFS route is known — do not default Max/hint to 1-hop (#587 / #1264). */
  const nativeSwapHopCount = defaultNativeWrapHopCount({
    operationsLength: nativeRouteInfo?.operations?.length,
    payIsNativeDenom,
    isDirectWrapOrUnwrap,
  })
  const nativeNeedsWrapInput = defaultNativeNeedsWrapInput({
    routeNeedsWrapInput: nativeRouteInfo?.needsWrapInput,
    payIsNativeDenom,
    isDirectWrapOrUnwrap,
  })
  const nativeNeedsUnwrapOutput =
    nativeRouteInfo?.needsUnwrapOutput ??
    (!!toToken && isNativeDenom(toToken) && wrapUnwrapType !== 'wrap' && wrapUnwrapType !== 'unwrap')

  const wrapMapperActive =
    !!WRAP_MAPPER_CONTRACT_ADDRESS &&
    (needsWrapCheck ||
      (isWrapOrUnwrap && wrapUnwrapType === 'unwrap') ||
      nativeRouteInfo?.needsUnwrapOutput ||
      wrapSolvePair?.needsUnwrapOutput ||
      false)

  const wrapMapperConfigQuery = useQuery({
    queryKey: ['wrapMapperConfig'],
    queryFn: queryWrapMapperConfig,
    enabled: false,
    staleTime: 30_000,
    refetchInterval: 30_000,
  })

  const unwrapBurnTaxDenom =
    wrapUnwrapType === 'unwrap' && isNativeDenom(toToken)
      ? toToken
      : (nativeRouteInfo?.needsUnwrapOutput || wrapSolvePair?.needsUnwrapOutput) && isNativeDenom(toToken)
        ? toToken
        : null

  const unwrapBurnTaxQuery = useQuery({
    queryKey: ['nativeTransferTax', unwrapBurnTaxDenom],
    queryFn: () => fetchNativeTransferTaxParams(unwrapBurnTaxDenom!),
    enabled: false,
    staleTime: 60_000,
  })

  const pausedQuery = useQuery({
    queryKey: ['wrapMapperPaused'],
    queryFn: queryPausedState,
    enabled: false,
    staleTime: 30_000,
  })

  const wrapMapperConfig = wrapMapperConfigQuery.data ?? null
  const wrapFeeBps = wrapMapperFeeBps(wrapMapperConfig, 'wrap')
  const unwrapFeeBps = wrapMapperFeeBps(wrapMapperConfig, 'unwrap')
  // Output intent stays output-sided even when only the server graph has a route (#1376).
  const canReverseQuote = fromToken !== toToken && allTokens.includes(fromToken) && allTokens.includes(toToken)
  const reverseQuoteActive = exactField === 'output' && !!fromToken && !!toToken
  const reverseAskRaw =
    reverseQuoteActive && receiveDecimals != null && isPositiveDecimalAmount(debouncedInputAmount)
      ? toRawAmount(debouncedInputAmount, receiveDecimals)
      : '0'

  const reverseQueryKey = [
    'reverseSimulation',
    fromToken,
    toToken,
    directPair?.contract_addr,
    reverseAskRaw,
    receiveDecimals,
    JSON.stringify(route),
    JSON.stringify(nativeRouteInfo),
    wrapUnwrapType,
    wrapFeeBps,
    unwrapFeeBps,
    wrapMapperConfigQuery.dataUpdatedAt,
    wrapMapperConfigQuery.isFetched,
    taxReceive.buyBps,
    address,
    slippageTolerance,
    debouncedHybridMaxMakers,
    debouncedBookInputHuman,
    unwrapBurnTaxQuery.dataUpdatedAt,
  ] as const
  const reverseQuery = useQuery<{ offerAmount: string; quote?: SwapSimData }, Error>({
    queryKey: reverseQueryKey,
    queryFn: async ({ signal }): Promise<{ offerAmount: string; quote?: SwapSimData }> => {
      if (!canReverseQuote) throw new Error('No eligible receive route found')
      if (debouncedBookInputHuman && isPositiveDecimalAmount(debouncedBookInputHuman)) {
        throw new Error('Clear the book leg before receive sizing.')
      }
      const quote = await quoteSwapReceiveViaSolver({
        fromToken,
        toToken,
        targetRaw: reverseAskRaw,
        maxMakerFills: debouncedHybridMaxMakers,
        slippageTolerancePercent: slippageTolerance,
        maxSpreadStr: slippagePercentToMaxSpread(slippageTolerance),
        quoteTrader: address ? { trader: address } : undefined,
        signal,
      })
      return { offerAmount: quote.offerAmount, quote }
    },
    // Deterministic liquidity/arithmetic failures never auto-retry or interval-refetch.
    retry: false,
    refetchOnWindowFocus: false,
    staleTime: localQuoteStaleTime,
    enabled: decimalsResolved && reverseQuoteActive && reverseAskRaw !== '0' && !taxReceive.isLoading,
    // Receive failures require explicit retry or a changed published generation.
    refetchInterval: (query) => (query.state.status === 'error' ? false : localQuoteRefetchInterval(query)),
  })

  const reverseRetry = useQueryManualRetry(reverseQueryKey, reverseQuery)
  useQuoteErrorRecovery({
    error: reverseQuery.error,
    intentKey: JSON.stringify([...reverseQueryKey, inputAmount, bookInputHuman, hybridMaxMakers]),
    currentIntent:
      reverseQuoteActive &&
      reverseQuery.isError &&
      !reverseQuery.isFetching &&
      inputAmount === debouncedInputAmount &&
      bookInputHuman === debouncedBookInputHuman &&
      hybridMaxMakers === debouncedHybridMaxMakers,
    tokenIn: fromToken,
    tokenOut: toToken,
    amount: reverseAskRaw,
    exactOutput: true,
    trader: address ?? undefined,
    maxMakerFills: debouncedHybridMaxMakers,
    maxSpread: slippagePercentToMaxSpread(slippageTolerance),
    poolOnly: true,
    refetch: reverseQuery.refetch,
  })
  const receiveTargetBelowPrecision =
    reverseQuoteActive &&
    receiveDecimals != null &&
    isPositiveDecimalAmount(inputAmount) &&
    toRawAmount(inputAmount, receiveDecimals) === '0'
  const receiveDecimalsUnavailable = reverseQuoteActive && !decimalsResolved && !decimalsPending
  const reverseIntentCurrent =
    reverseQuoteActive &&
    reverseAskRaw !== '0' &&
    inputAmount === debouncedInputAmount &&
    bookInputHuman === debouncedBookInputHuman &&
    hybridMaxMakers === debouncedHybridMaxMakers
  const reversePricingDataPending = useQuotePricingDataPending({
    intentKey: JSON.stringify([...reverseQueryKey, inputAmount, bookInputHuman, hybridMaxMakers]),
    currentIntent: reverseIntentCurrent,
    error: reverseQuery.error,
    hasValidatedQuote:
      !!reverseQuery.data &&
      !reverseQuery.isError &&
      !reverseQuery.isFetching &&
      !isExpiredLocalQuote(reverseQuery.data),
    expired: isExpiredLocalQuote(reverseQuery.data),
  })
  const reverseOfferRaw = reverseQuery.isError ? null : (reverseQuery.data?.offerAmount ?? null)
  const reverseQuoteReady =
    reverseQuoteActive &&
    inputAmount === debouncedInputAmount &&
    isPositiveDecimalAmount(inputAmount) &&
    !!reverseOfferRaw &&
    isPositiveRawAmount(reverseOfferRaw) &&
    !taxReceive.isLoading &&
    !reverseQuery.isFetching &&
    !isExpiredLocalQuote(reverseQuery.data)
  const rawInputAmount = reverseQuoteActive ? (reverseQuoteReady ? reverseOfferRaw : '0') : typedPayRaw
  const debouncedRawInputAmount = reverseQuoteActive
    ? reverseQuoteReady
      ? reverseOfferRaw
      : '0'
    : debouncedTypedPayRaw

  const bookLegMaxResult = useMemo(() => {
    if (!balanceQuery.data || rawInputAmount === '0' || offerDecimals == null) {
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

  const rateLimitQuery = useQuery({
    queryKey: ['rateLimit', wrapDenom, rawInputAmount],
    queryFn: () => {
      if (!wrapDenom) throw new Error('No denom')
      return checkRateLimitExceeded(wrapDenom, rawInputAmount)
    },
    enabled: false,
    staleTime: 15_000,
  })

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

  const checkIndexerRoute = useCallback(async () => {
    if (!fromToken || !toToken) return
    setIndexerRouteError(null)
    setIndexerRouteResult(null)
    if (!rawInputAmount || rawInputAmount === '0') {
      setIndexerRouteError('Enter an amount to compare routes.')
      return
    }
    setIndexerRouteLoading(true)
    try {
      const res = await getRouteSolve(fromToken, toToken, rawInputAmount, {
        maxSpread: slippagePercentToMaxSpread(slippageTolerance),
        maxMakerFills: hybridMaxMakers,
        trader: address ?? undefined,
        poolOnly: isNativeDenom(fromToken) || isNativeDenom(toToken),
      })
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
  }, [fromToken, toToken, rawInputAmount, slippageTolerance, hybridMaxMakers, address])

  const simQueryKey = useMemo(
    () =>
      [
        'simulation',
        reverseQuoteActive ? reverseQuery.dataUpdatedAt : null,
        fromToken,
        toToken,
        debouncedRawInputAmount,
        offerDecimals,
        receiveDecimals,
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
        routerTaxFunding.data,
      ] as const,
    [
      reverseQuoteActive,
      reverseQuery.dataUpdatedAt,
      fromToken,
      toToken,
      debouncedRawInputAmount,
      offerDecimals,
      receiveDecimals,
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
      routerTaxFunding.data,
    ]
  )

  const quoteTrader = useMemo(() => (address ? { trader: address } : undefined), [address])

  const simQuery = useQuery<SwapSimData>({
    queryKey: simQueryKey,
    placeholderData: keepPreviousData,
    queryFn: async ({ signal }): Promise<SwapSimData> => {
      if (debouncedRawInputAmount === '0') throw new Error(LOCAL_ROUTE_UNAVAILABLE)
      const split =
        debouncedBookInputHuman && isPositiveDecimalAmount(debouncedBookInputHuman)
          ? getDirectHybridBookSplit({
              isDirect,
              fromToken,
              bookInputHuman: debouncedBookInputHuman,
              rawInputAmount: debouncedRawInputAmount,
              hybridMaxMakers: debouncedHybridMaxMakers,
              payDecimals: offerDecimals,
            })
          : null
      if (debouncedBookInputHuman && isPositiveDecimalAmount(debouncedBookInputHuman) && !split?.willSubmitHybrid)
        throw new Error('This book split is unavailable. Check the book amount.')
      if (reverseQuoteActive && reverseQuery.data?.quote) return reverseQuery.data.quote
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
      return { ...quoted }
    },
    enabled:
      decimalsResolved &&
      hasRoute &&
      isPositiveDecimalAmount(debouncedInputAmount) &&
      debouncedRawInputAmount !== '0' &&
      (!reverseQuoteActive || reverseQuoteReady),
    // Receive refresh belongs to reverseQuery so offer and operations refresh together.
    retry: false,
    refetchOnWindowFocus: false,
    staleTime: localQuoteStaleTime,
    refetchInterval: reverseQuoteActive ? false : localQuoteRefetchInterval,
  })

  const simRetry = useQueryManualRetry(simQueryKey, simQuery)

  const routeSolveProgressEnabled =
    !isWrapOrUnwrap &&
    !reverseQuoteActive &&
    !isPositiveDecimalAmount(debouncedBookInputHuman) &&
    (wrapSolvePair
      ? wrapSolvePair.tokenIn.startsWith('terra1') && wrapSolvePair.tokenOut.startsWith('terra1')
      : !nativeRouteInfo && fromToken.startsWith('terra1') && toToken.startsWith('terra1')) &&
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
    knownDiscountBps: undefined,
    maxMakerFills: debouncedHybridMaxMakers,
    maxSpread: slippagePercentToMaxSpread(slippageTolerance),
    poolOnly: isNativeDenom(fromToken) || isNativeDenom(toToken),
  })

  // Settled for *current* sim key only — placeholder keepPreviousData is prior-key stale (#496).
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
  const forwardPricingDataPending = useQuotePricingDataPending({
    intentKey: JSON.stringify([...simQueryKey, rawInputAmount, bookInputHuman, hybridMaxMakers]),
    currentIntent:
      !reverseQuoteActive && debouncedRawInputAmount !== '0' && simInputIntentCurrent && !simQuery.isPlaceholderData,
    error: simQuery.error,
    hasValidatedQuote: hasSettledSimQuote && !simQuery.isFetching && !simQuery.isError,
    expired: isExpiredLocalQuote(simQuery.data),
  })
  const pricingDataPending = reverseQuoteActive ? reversePricingDataPending : forwardPricingDataPending
  const payInputsPendingForReceive = rawInputAmount !== debouncedRawInputAmount
  const showSimReceiveCalculating =
    pricingDataPending ||
    shouldShowSimReceiveCalculating(
      simQuery.isFetching,
      hasSettledSimQuote,
      simQuery.isPlaceholderData,
      payInputsPendingForReceive
    )

  const simLoadingLabel = pricingDataPending
    ? 'Loading price…'
    : resolveSimQuoteLoadingLabel(
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

  const {
    submitPayRaw,
    simData,
    minReceived,
    isQuoteStale: simQuoteStale,
    snapshottedHybrid,
  } = useSubmitAlignedSimQuote({
    rawInputAmount,
    debouncedRawInputAmount,
    simQuery:
      (reverseQuoteActive && !reverseQuoteReady) || isIndexerLocalQuoteRejection(simQuery.error)
        ? { ...simQuery, data: undefined }
        : simQuery,
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
    usesRouter:
      !!simData?.executionMetadata ||
      simData?.taxFundingVersion === 1 ||
      communityTaxExecuteUsesRouter(simData?.indexerOperations?.length, isMultiHop),
    sellBps: taxSell.sellBps,
  })
  const extraDebitUsesRouter =
    !!simData?.executionMetadata ||
    simData?.taxFundingVersion === 1 ||
    communityTaxExecuteUsesRouter(simData?.indexerOperations?.length, isMultiHop)
  const taxPreviewMaxSpread = slippagePercentToMaxSpread(slippageTolerance)
  const taxPreviewQuery = useMemo(
    () =>
      address && fromToken.startsWith('terra1')
        ? resolveCommunityTaxPreviewQuery({
            wallet: address,
            payToken: fromToken,
            usesRouter: extraDebitUsesRouter,
            directPairAddr: directPair?.contract_addr,
            routeOps: simData?.indexerOperations,
            pairs,
            maxSpread: taxPreviewMaxSpread,
          })
        : null,
    [
      address,
      fromToken,
      extraDebitUsesRouter,
      directPair?.contract_addr,
      simData?.indexerOperations,
      pairs,
      taxPreviewMaxSpread,
    ]
  )
  const taxPreview = useCommunityTaxPreviewDebit({
    token: fromToken.startsWith('terra1') ? fromToken : null,
    amount: rawInputAmount,
    enabled: !extraDebitUsesRouter && taxSell.isTaxToken && taxSell.sellBps != null && taxSell.sellBps > 0,
    previewQuery: taxPreviewQuery,
  })
  const extraDebitGate = extraDebitSubmitGate({
    balanceUnresolved: balanceQuery.isError,
    declaredRaw: tryParseBigInt(rawInputAmount),
    balanceRaw: balanceQuery.data !== undefined ? tryParseBigInt(balanceQuery.data) : null,
    debitRaw: taxPreview.debitRaw,
    sellBps: extraDebitSellBpsForExecute(taxSell.sellBps, extraDebitUsesRouter),
    extraDebitUnresolved: taxSell.extraDebitUnresolved || taxPreview.previewUnresolved,
    isNativePay: payIsNativeUluna || !fromToken.startsWith('terra1'),
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
      pairs: [...(simData?.indexerPairs ?? []), ...pairs],
      directPair,
      fromToken,
      toToken,
    })
    return { tokens: [...tokens], pairAddresses }
  }, [route, nativeRouteInfo, simData?.indexerOperations, simData?.indexerPairs, directPair, pairs, fromToken, toToken])

  const tradingBlacklist = useTradingBlacklist({
    wallet: address,
    tokens: swapBlacklistProbe.tokens,
    pairAddress: swapBlacklistProbe.pairAddresses.length === 1 ? swapBlacklistProbe.pairAddresses[0] : null,
    pairs: swapBlacklistProbe.pairAddresses.length > 1 ? swapBlacklistProbe.pairAddresses : null,
    enabled: false,
  })

  const pairPaused = usePairPaused({
    enabled: false,
    pairAddress: swapBlacklistProbe.pairAddresses.length === 1 ? swapBlacklistProbe.pairAddresses[0] : null,
    pairAddresses: swapBlacklistProbe.pairAddresses.length > 1 ? swapBlacklistProbe.pairAddresses : null,
  })
  const isPairPaused = pairPaused.isPaused
  const pairCodeIdFreeze = usePairCodeIdFreeze({
    enabled: false,
    pairAddress: swapBlacklistProbe.pairAddresses.length === 1 ? swapBlacklistProbe.pairAddresses[0] : null,
    pairAddresses: swapBlacklistProbe.pairAddresses.length > 1 ? swapBlacklistProbe.pairAddresses : null,
  })
  const isPairCodeIdFrozen = pairCodeIdFreeze.isFrozen

  const swapMutation = useTerraBroadcastMutation({
    toastSuccess: 'Swap submitted.',
    toastOnError: false,
    mutationFn: async () => {
      if (!address || !inputAmount) throw new Error('Missing parameters')
      if (isPairPaused || isWrapPaused) throw new Error('Trading is paused.')
      if (isPairCodeIdFrozen) throw new Error('Trading is temporarily unavailable.')
      if (tradingBlacklist.blocked) throw new Error('Trading is restricted.')
      if (!decimalsResolved || offerDecimals == null || receiveDecimals == null) {
        throw new Error('Token decimals unavailable')
      }
      if (extraDebitGate.blockSubmit) {
        throw new Error(extraDebitGate.insufficientBalance ? 'Insufficient Balance' : TAX_FUNDING_UNRESOLVED_MESSAGE)
      }
      assertSubmitQuotePayRawAligned(rawInputAmount, debouncedRawInputAmount)
      if (snapshottedHybrid) {
        assertSubmitHybridAligned(hybridStaleLive, snapshottedHybrid)
      }
      if (reverseQuoteActive && !reverseQuoteReady) throw new Error('Quote unavailable')
      if (simQuoteStale || simQuery.isPlaceholderData || simQuery.isFetching || simQuery.isError) {
        throw new Error('Quote unavailable')
      }
      if (!simData) throw new Error('Quote unavailable')
      if (simData.executionBlocker || simData.executionMetadata?.execution_supported === false)
        throw new Error(simData.executionBlocker ?? LOCAL_ROUTE_UNAVAILABLE)
      if (routeSlippageBlocked) throw new Error('Price difference is too high')
      if (
        (expectedSlippagePct ?? 0) > SWAP_FUNDED_HIGH_IMPACT_PCT &&
        impactAcceptedQuoteRef.current !== impactReviewKey
      )
        throw new Error('Confirm the price difference first.')
      impactAcceptedQuoteRef.current = null
      if (simData.routePreflight?.anyHopExceedsMaxSpread) {
        throw new Error('Quote exceeds your price protection.')
      }
      const payRaw = submitPayRaw
      // Recheck at mutation entry too: a queued submit must not bypass the
      // displayed native pay+fee gate after cached balance/coverage changes.
      if (swapNetworkFeeEstimate.coverageError) throw new Error(swapNetworkFeeEstimate.coverageError)
      const submitGasGate = evaluateSwapNativeGasGate(
        inputAmount,
        offerDecimals,
        payIsNativeUluna,
        payRaw,
        payIsNativeUluna ? balanceQuery : ulunaBalanceQuery,
        swapNetworkFeeEstimate.feeUluna
      )
      if (!submitGasGate.canSubmit) throw new Error(submitGasGate.userMessage ?? 'Need LUNC for network fee')
      // Display min uses post-tax `return_amount`; router R3 checks post-fee pre-tax (#512).
      const submitMinReceived =
        simData.executionMetadata?.policy === 'wrapper_only'
          ? simData.executionMetadata.minimum_receive
          : simData.executionMetadata?.wrapper != null
            ? simData.executionMetadata.wrapper.minimum_receive_base
            : simData.routerMinReceiveBase != null && simData.routerMinReceiveBase !== simData.return_amount
              ? applySlippagePercentFloor(simData.routerMinReceiveBase, slippageTolerance)
              : minReceived
      const maxSpread = slippagePercentToMaxSpread(slippageTolerance)

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
      if (simData.executionMetadata.wrapper)
        return executeLocalNativeRoute(
          address,
          simData.executionMetadata,
          idxOps,
          maxSpread,
          submitMinReceived!,
          deadline
        )
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
  // The offline wrapper cache is unknown before a current local quote exists.
  // Empty/debouncing/loading intent is not evidence of wrapper unavailability.
  // These presentation predicates never replace execution or budget validation.
  const hasCurrentWrapperContext =
    hasSettledSimQuote && !!simData?.executionMetadata?.wrapper && (!reverseQuoteActive || reverseQuoteReady)
  const hasRawWrapperQuoteIntent = reverseQuoteActive ? reverseAskRaw !== '0' : rawInputAmount !== '0'
  const wrapperQuotePending =
    inputAmount !== debouncedInputAmount ||
    !simInputIntentCurrent ||
    simQuery.isPlaceholderData ||
    simQuery.isLoading ||
    simQuery.isFetching ||
    (hasRawWrapperQuoteIntent &&
      isExpiredLocalQuote(simQuery.data) &&
      !simQuery.isError &&
      (!reverseQuoteActive || !reverseQuery.isError)) ||
    pricingDataPending ||
    (reverseQuoteActive && reverseAskRaw !== '0' && !reverseQuoteReady && !reverseQuery.isError)
  const showWrapTreasuryBlocker = hasPositiveInputAmount && !hasCurrentWrapperContext && wrapTreasuryMismatch
  const showWrapSafetyBlocker =
    hasPositiveInputAmount &&
    hasRawWrapperQuoteIntent &&
    !hasCurrentWrapperContext &&
    wrapSafetyUnavailable &&
    !wrapperQuotePending &&
    !simQuery.isError &&
    (!reverseQuoteActive || !reverseQuery.isError)
  const waitingForWrapperQuote =
    hasPositiveInputAmount && !hasCurrentWrapperContext && wrapSafetyUnavailable && wrapperQuotePending
  const simQuoteUnavailable =
    !pricingDataPending &&
    !isWrapOrUnwrap &&
    hasPositiveInputAmount &&
    (simQuery.isError || (!simQuery.isLoading && !simData))
  const showSimRetryError =
    simQuery.isError &&
    !pricingDataPending &&
    !isWrapOrUnwrap &&
    hasPositiveInputAmount &&
    simInputIntentCurrent &&
    !simQuery.isFetching &&
    !indexerOutage &&
    !isIndexerPairNotFoundError(simQuery.error)

  useQuoteErrorRecovery({
    error: simQuery.error,
    intentKey: JSON.stringify([...simQueryKey, rawInputAmount, bookInputHuman, hybridMaxMakers]),
    currentIntent: !reverseQuoteActive && simQuery.isError && !simQuery.isFetching && simInputIntentCurrent,
    tokenIn: fromToken,
    tokenOut: toToken,
    amount: debouncedRawInputAmount,
    trader: address ?? undefined,
    maxMakerFills: debouncedHybridMaxMakers,
    maxSpread: slippagePercentToMaxSpread(slippageTolerance),
    poolOnly: isNativeDenom(fromToken) || isNativeDenom(toToken),
    refetch: simQuery.refetch,
  })
  const executableOutput = !simData?.executionBlocker && simData?.executionMetadata?.execution_supported !== false
  const outputAmount = executableOutput ? (simData?.return_amount ?? '') : ''
  const commissionAmount = simData?.commission_amount ?? ''

  const hopSpreadPercent = simData
    ? simData.routePreflight != null
      ? simData.routePreflight.worstSpreadPercent
      : spreadPercentFromRawSim(simData.return_amount, simData.commission_amount, simData.spread_amount)
    : null

  const expectedSlippagePct = simData
    ? resolveSwapExpectedSlippagePercent(simData.routeSlippagePercent, hopSpreadPercent)
    : null

  const impactReviewKey = JSON.stringify([
    address,
    fromToken,
    toToken,
    rawInputAmount,
    bookInputHuman,
    hybridMaxMakers,
    slippageTolerance,
    deadlineSeconds,
    expertMode,
    simData?.routeSlippagePercent,
    submitPayRaw,
    simData?.executionMetadata,
    simData?.executionOperations,
    simData?.return_amount,
  ])
  useEffect(() => {
    if (impactReviewedQuoteRef.current !== impactReviewKey) {
      impactAcceptedQuoteRef.current = null
      setShowImpactConfirm(false)
    }
  }, [impactReviewKey])

  useRouteQuotePromotion({
    enabled:
      routeSolveProgressEnabled &&
      !reverseQuoteActive &&
      !isPositiveDecimalAmount(bookInputHuman) &&
      hasSettledSimQuote &&
      !simQuery.isFetching &&
      !simQuery.isError &&
      !showImpactConfirm &&
      !swapMutation.isPending,
    intentKey: JSON.stringify(simQueryKey),
    tokenIn: fromToken,
    tokenOut: toToken,
    amount: debouncedRawInputAmount,
    trader: quoteTrader?.trader,
    maxMakerFills: debouncedHybridMaxMakers,
    maxSpread: slippagePercentToMaxSpread(slippageTolerance),
    poolOnly: isNativeDenom(fromToken) || isNativeDenom(toToken),
    quote: simQuery.data,
    refetch: simQuery.refetch,
  })

  const priceImpact = expectedSlippagePct != null ? expectedSlippagePct.toFixed(2) : hopSpreadPercent

  const theaterRouteQuote = isTheaterRouteQuote(expectedSlippagePct)
  const routeSlippageBlocked = swapRouteSlippageBlocksSubmit(expectedSlippagePct, expertMode)

  const directHybridBookSplit = useMemo(
    () =>
      getDirectHybridBookSplit({
        isDirect,
        fromToken,
        bookInputHuman: debouncedBookInputHuman,
        rawInputAmount: debouncedRawInputAmount,
        hybridMaxMakers: debouncedHybridMaxMakers,
        payDecimals: offerDecimals,
      }),
    [isDirect, fromToken, debouncedBookInputHuman, debouncedRawInputAmount, debouncedHybridMaxMakers, offerDecimals]
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
        nativeWrapExecute: !!wrapSolvePair,
        indexerOperations: simData?.indexerOperations,
        clientRoute: route,
        isDirect,
        isMultiHop,
      }),
    [isWrapOrUnwrap, nativeRouteInfo, wrapSolvePair, simData?.indexerOperations, route, isDirect, isMultiHop]
  )

  const showClientBfsFallbackLabel = swapSubmitRouteSource === 'client_bfs'
  const showRouteIntermediateReconciledLabel = !!simData?.indexerRouteIntermediateReconciled
  const showDirectHybridAmountReconciledLabel = !!simData?.indexerAmountReconciled

  const swapNetworkFeeHints = useMemo(() => {
    const cw20HopCount = simData?.indexerOperations?.length ?? route?.length ?? 1
    const indexerHopCount = simData?.indexerOperations?.length ?? 0
    const wrapOrNative = Boolean(isWrapOrUnwrap || wrapSolvePair || nativeRouteInfo)
    const cw20RouterOperations =
      !wrapOrNative &&
      (indexerHopCount >= 2 ||
        !!simData?.executionMetadata ||
        (simData?.taxFundingVersion === 1 && indexerHopCount >= 1))
        ? simData?.indexerOperations
        : undefined
    return {
      taxFundingVersion: simData?.executionMetadata
        ? (simData.executionMetadata.tax_funding_version ?? undefined)
        : (simData?.taxFundingVersion ?? (routerTaxFunding.data === 1 ? 1 : undefined)),
      isDirectWrap: wrapUnwrapType === 'wrap',
      isDirectUnwrap: wrapUnwrapType === 'unwrap',
      needsWrapInput: nativeNeedsWrapInput || Boolean(wrapSolvePair?.needsWrapInput),
      needsUnwrapOutput: nativeNeedsUnwrapOutput || Boolean(wrapSolvePair?.needsUnwrapOutput),
      hopCount:
        indexerHopCount > 0
          ? indexerHopCount
          : wrapOrNative
            ? (nativeRouteInfo?.operations?.length ?? nativeSwapHopCount)
            : cw20HopCount,
      cw20DirectPair: !!isDirect && !wrapOrNative && !simData?.executionMetadata && simData?.taxFundingVersion !== 1,
      cw20Hybrid: !!isDirect && isPositiveDecimalAmount(bookInputHuman.trim()),
      cw20HybridParams: simData?.indexerOperations?.[0]?.terra_swap.hybrid,
      localExecution: !!simData?.executionMetadata,
      nativeRouterOperations: wrapOrNative ? simData?.indexerOperations : undefined,
      cw20RouterOperations,
    }
  }, [
    wrapUnwrapType,
    nativeNeedsWrapInput,
    nativeNeedsUnwrapOutput,
    nativeRouteInfo,
    nativeSwapHopCount,
    isWrapOrUnwrap,
    wrapSolvePair,
    isDirect,
    bookInputHuman,
    simData?.indexerOperations,
    simData?.taxFundingVersion,
    simData?.executionMetadata,
    routerTaxFunding.data,
    route?.length,
  ])
  const swapNetworkFeeEstimate = useMemo(() => tryEstimateSwapNetworkFee(swapNetworkFeeHints), [swapNetworkFeeHints])

  const payMaxResult = useMemo(() => {
    if (!balanceQuery.data || offerDecimals == null) {
      return { human: '0', spendableRaw: 0n, cappedByGas: false, reserveUluna: 0n }
    }
    return computeMaxSpendableHumanAmount({
      balanceRaw: balanceQuery.data,
      decimals: offerDecimals,
      assetIsNativeUluna: payIsNativeUluna,
      context: payIsNativeUluna ? 'swap_native' : 'swap_cw20',
      executionFeeUluna: swapNetworkFeeEstimate.coverageError
        ? BigInt(balanceQuery.data)
        : swapNetworkFeeEstimate.feeUluna,
      extraDebitSellBps: extraDebitSellBpsForExecute(taxSell.sellBps, isMultiHop || routerTaxFunding.data === 1),
    })
  }, [
    balanceQuery.data,
    offerDecimals,
    payIsNativeUluna,
    taxSell.sellBps,
    isMultiHop,
    routerTaxFunding.data,
    swapNetworkFeeEstimate,
  ])

  const swapGasGate =
    offerDecimals == null
      ? { canSubmit: false, userMessage: null as string | null, tone: 'none' as const }
      : evaluateSwapNativeGasGate(
          inputAmount,
          offerDecimals,
          payIsNativeUluna,
          rawInputAmount,
          payIsNativeUluna ? balanceQuery : ulunaBalanceQuery,
          swapNetworkFeeEstimate.feeUluna
        )

  const insufficientBalance = extraDebitGate.insufficientBalance

  const payAcquireGuidance = useSwapPayAcquireGuidance({
    walletConnected: isWalletConnected,
    address,
    hasPositivePay: hasPositiveInputAmount && decimalsResolved,
    hasSettledQuote: hasSettledSimQuote && decimalsResolved,
    payAsset: fromToken,
    paySymbol: getTokenDisplaySymbol(fromToken),
    payDecimals: offerDecimals ?? 6,
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
  } else if (showWrapTreasuryBlocker) {
    buttonText = WRAP_TREASURY_MISCONFIGURED_CTA
    buttonDisabled = true
  } else if (showWrapSafetyBlocker) {
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
  } else if (!decimalsResolved) {
    buttonText = decimalsPending ? 'Loading decimals…' : 'Token decimals unavailable'
    buttonDisabled = true
  } else if (receiveTargetBelowPrecision) {
    buttonText = 'Increase receive amount'
    buttonDisabled = true
  } else if (pricingDataPending) {
    buttonText = 'Wait for price'
    buttonDisabled = true
  } else if (isRateLimitExceeded) {
    buttonText = 'Rate Limit Exceeded'
    buttonDisabled = true
  } else if (insufficientBalance) {
    buttonText = 'Insufficient Balance'
    buttonDisabled = true
  } else if (extraDebitGate.blockSubmit) {
    buttonText = defaultActionLabel
    buttonDisabled = true
  } else if (waitingForWrapperQuote) {
    buttonText = simLoadingLabel
    buttonDisabled = true
  } else if (swapNetworkFeeEstimate.coverageError) {
    buttonText = 'Network fee unavailable'
    buttonDisabled = true
  } else if (hasPositiveInputAmount && !swapGasGate.canSubmit) {
    buttonText = swapGasGate.userMessage ?? 'Need LUNC for network fee'
    buttonDisabled = true
  } else if (reverseQuoteActive && !reverseQuoteReady) {
    buttonText = receiveTargetBelowPrecision
      ? 'Increase receive amount'
      : reverseQuery.isError || receiveDecimalsUnavailable
        ? 'Quote unavailable'
        : 'Calculating...'
    buttonDisabled = true
  } else if (simQuoteUnavailable) {
    buttonText = 'Quote unavailable'
    buttonDisabled = true
  } else if (simData?.executionBlocker) {
    buttonText = 'Trade unavailable'
    buttonDisabled = true
  } else if (routeSlippageBlocked) {
    buttonText = 'Price difference is too high'
    buttonDisabled = true
  } else if (simData?.routePreflight?.anyHopExceedsMaxSpread) {
    buttonText = 'Quote exceeds price protection'
    buttonDisabled = true
  } else if (simQuery.isLoading || simQuoteStale) {
    buttonText = simLoadingLabel
    buttonDisabled = true
  } else if (swapMutation.isPending) {
    buttonText = terraBroadcastPendingButtonLabel(swapMutation.phase, true, defaultActionLabel, defaultPendingLabel)
    buttonDisabled = true
  } else if (showImpactConfirm) {
    buttonText = `Confirm ${defaultActionLabel}`
    buttonDisabled = false
  }

  const primaryNotice = firstTradingNotice([
    ['tx', swapMutation.isError && !isPreSignTradingError(swapMutation.error)],
    ['paused', isPairPaused || isWrapPaused],
    ['frozen', isPairCodeIdFrozen],
    ['restricted', tradingBlacklist.blocked],
    ['route', !hasRoute && hasPositiveInputAmount],
    ['precision', receiveTargetBelowPrecision],
    ['pricing', pricingDataPending],
    ['receive', reverseQuoteActive && (reverseQuery.isError || receiveDecimalsUnavailable)],
    ['retry', showSimRetryError],
    ['outage', !!indexerOutage],
    ['wrapper', !!simData && !executableOutput],
    ['rate', isRateLimitExceeded],
    ['config', showWrapTreasuryBlocker || showWrapSafetyBlocker],
    ['acquire', isWalletConnected && payAcquireGuidance.kind.startsWith('insufficient')],
    [
      'funding',
      !!swapNetworkFeeEstimate.coverageError ||
        (isWalletConnected && (extraDebitGate.blockSubmit || (hasPositiveInputAmount && !swapGasGate.canSubmit))),
    ],
    ['hop', simData?.routePreflight?.anyHopExceedsMaxSpread === true],
    [
      'price',
      // Background refresh blocks signing but retains the current fresh quote
      // and its factual price notice. Intent changes and expiry still clear both.
      hasSettledSimQuote &&
        !simQuoteUnavailable &&
        expectedSlippagePct != null &&
        expectedSlippagePct > SWAP_FUNDED_HIGH_IMPACT_PCT,
    ],
    ['tx', swapMutation.isError],
  ] as const)

  if (
    primaryNotice === 'tx' &&
    showImpactConfirm &&
    !buttonDisabled &&
    expectedSlippagePct != null &&
    expectedSlippagePct > SWAP_FUNDED_HIGH_IMPACT_PCT
  )
    buttonText = `Confirm ${defaultActionLabel} (${formatTradingPriceDifference(expectedSlippagePct)}%)`

  const lastErrorIntentRef = useRef(impactReviewKey)
  const { error: lastMutationError, reset: resetMutationError } = swapMutation
  useEffect(() => {
    const changed = lastErrorIntentRef.current !== impactReviewKey
    lastErrorIntentRef.current = impactReviewKey
    if (changed && isPreSignTradingError(lastMutationError)) resetMutationError()
  }, [impactReviewKey, lastMutationError, resetMutationError])

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

  const reversePayHuman =
    reverseQuoteReady && reverseOfferRaw && offerDecimals != null ? fromRawAmount(reverseOfferRaw, offerDecimals) : ''
  const payInputValue = reverseQuoteActive ? reversePayHuman : inputAmount
  const quotedReceiveHuman = theaterRouteQuote
    ? '—'
    : outputAmount && receiveAssetInfo && receiveDecimals != null
      ? fromRawAmount(outputAmount, receiveDecimals)
      : ''
  const receiveInputValue = reverseQuoteActive ? inputAmount : quotedReceiveHuman
  const showReversePayCalculating =
    reverseQuoteActive &&
    !reverseQuoteReady &&
    (!reverseQuery.isError || reversePricingDataPending) &&
    !receiveTargetBelowPrecision &&
    !receiveDecimalsUnavailable &&
    isPositiveDecimalAmount(inputAmount)

  const applyTypedAmount = (value: string, field: 'input' | 'output') => {
    if (value.length <= 24 && (value === '' || /^\d*\.?\d*$/.test(value))) {
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
    <LocalQuoteTokenMetadata.Provider value={true}>
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
                    ariaLabel={swapShareAriaLabel(fromToken, toToken)}
                    buttonContent={<SwapSharePairLabel payId={fromToken} receiveId={toToken} />}
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

            {primaryNotice === 'outage' && !pricingDataPending && (
              <MarketDataServiceOutageBanner
                testId="swap-market-data-outage-banner"
                title={MARKET_DATA_SERVICE_OUTAGE_TITLE}
                lead={SWAP_MARKET_DATA_OUTAGE_LEAD}
                onRetry={simRetry.retry}
              />
            )}

            {primaryNotice === 'precision' && (
              <p role="alert" className="alert-error text-xs mb-4" data-testid="swap-receive-amount-error">
                Receive target is below token precision. Increase the receive amount.
              </p>
            )}

            {pricingDataPending && <QuoteDataLoadingProgress testId="swap-price-data-loading" />}

            {primaryNotice === 'receive' &&
              reverseQuoteActive &&
              reverseQuery.isError &&
              isIndexerRouteValidationError(reverseQuery.error) &&
              hasPositiveInputAmount &&
              inputAmount === debouncedInputAmount &&
              !reverseQuery.isFetching && (
                <p className="alert-error text-xs mb-4" role="alert" data-testid="swap-receive-quote-error">
                  {humanizeUserFacingErrorFromUnknown(reverseQuery.error)}
                </p>
              )}
            {primaryNotice === 'receive' &&
              reverseQuoteActive &&
              (reverseQuery.isError || receiveDecimalsUnavailable) &&
              hasPositiveInputAmount &&
              inputAmount === debouncedInputAmount &&
              !reverseQuery.isFetching &&
              !reversePricingDataPending &&
              !isIndexerRouteValidationError(reverseQuery.error) && (
                <RetryError
                  data-testid="swap-receive-quote-retry-error"
                  message={
                    receiveDecimalsUnavailable
                      ? 'Token decimals are unavailable. Retry token metadata to size this receive target.'
                      : humanizeUserFacingErrorFromUnknown(reverseQuery.error)
                  }
                  isRetrying={reverseRetry.isRetrying}
                  onRetry={() => {
                    if (receiveDecimalsUnavailable) {
                      void queryClient.invalidateQueries({ queryKey: ['cw20-token-info-decimals'] })
                      void queryClient.invalidateQueries({ queryKey: ['indexer-tokens-list'] })
                    } else if ((isWrapOrUnwrap || wrapSolvePair) && wrapMapperConfig == null)
                      void wrapMapperConfigQuery.refetch()
                    else reverseRetry.retry()
                  }}
                />
              )}

            {primaryNotice === 'retry' &&
              showSimRetryError &&
              (isIndexerRouteValidationError(simQuery.error) ? (
                <p className="alert-error text-xs mb-4" role="alert" data-testid="swap-quote-error">
                  {humanizeUserFacingErrorFromUnknown(simQuery.error)}
                </p>
              ) : (
                <RetryError
                  data-testid="swap-quote-retry-error"
                  message={humanizeUserFacingErrorFromUnknown(simQuery.error)}
                  isRetrying={simRetry.isRetrying}
                  onRetry={simRetry.retry}
                />
              ))}

            {primaryNotice === 'wrapper' && simData && !executableOutput && (
              <p className="alert-error text-xs mb-4" role="alert" data-testid="swap-wrapper-quote-error">
                This conversion cannot protect your minimum receive.
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
                    <p className="label-glass mb-3">Number display</p>
                    <div className="flex flex-wrap gap-2" role="group" aria-label="Number display">
                      {(['compact', 'full'] as const).map((mode) => (
                        <button
                          key={mode}
                          type="button"
                          aria-pressed={numberDisplayMode === mode}
                          data-testid={`swap-number-display-${mode}`}
                          onClick={() => {
                            sounds.playButtonPress()
                            setNumberDisplayMode(mode)
                          }}
                          className={`tab-glass min-h-[44px] min-w-[44px] !text-xs !px-3 ${
                            numberDisplayMode === mode ? 'tab-glass-active' : 'tab-glass-inactive'
                          }`}
                        >
                          {mode === 'compact' ? 'Compact' : 'Full'}
                        </button>
                      ))}
                    </div>
                    <p
                      className="mt-2 text-xs break-words"
                      style={{ color: 'var(--ink-subtle)' }}
                      data-testid="swap-number-display-preview"
                      aria-live="polite"
                    >
                      Example: {formatNum(1234567, 4, numberDisplayMode)}
                    </p>
                  </div>
                  <div className="mt-4 pt-3 border-t" style={{ borderColor: 'var(--border-subtle)' }}>
                    <label
                      className="flex items-center gap-2 text-xs cursor-pointer"
                      title={`Trades with a price difference above ${SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT}% require Expert Mode.`}
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
                  aria-busy={pricingDataPending || showReversePayCalculating || undefined}
                />
                {isWalletConnected && offerDecimals != null && (
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
                  <label htmlFor={swapYouReceiveAmountInputId} className="label-glass !mb-0 sm:pt-1">
                    You Receive
                  </label>
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
                <input
                  id={swapYouReceiveAmountInputId}
                  type="text"
                  inputMode="decimal"
                  value={reverseQuoteActive ? inputAmount : showSimReceiveCalculating ? '' : receiveInputValue}
                  onChange={(e) => applyTypedAmount(e.target.value, 'output')}
                  placeholder="0.00"
                  maxLength={24}
                  className="swap-io-amount-input w-full text-[1.75rem] sm:text-2xl font-medium bg-transparent"
                  style={{ color: 'var(--ink)' }}
                  data-testid="swap-you-receive"
                  aria-busy={pricingDataPending || (showSimReceiveCalculating && !reverseQuoteActive) || undefined}
                />
                {showQuoteOnly && (
                  <p className="text-xs mt-1" style={{ color: 'var(--ink-subtle)' }} data-testid="swap-quote-only">
                    Quote only
                  </p>
                )}
                {showSimReceiveCalculating && !pricingDataPending && (
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
                      <p
                        className="uppercase tracking-wide font-semibold mb-0.5"
                        style={{ color: 'var(--ink-subtle)' }}
                      >
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
                            {directHybridSettingsExec.poolRaw} (pool raw) · {directHybridSettingsExec.bookRaw} (book
                            raw)
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
            {simData && executableOutput && (
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
                      {receiveAssetInfo && receiveDecimals != null
                        ? formatTokenAmount(minReceived!, receiveDecimals)
                        : minReceived}
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
                          {(() => {
                            const a0 = poolQuery.data.assets[0]
                            const a1 = poolQuery.data.assets[1]
                            const id0 = assetInfoLabel(a0.info)
                            const id1 = assetInfoLabel(a1.info)
                            const d0 =
                              id0 === fromToken
                                ? offerDecimals
                                : id0 === toToken
                                  ? receiveDecimals
                                  : registrySwapDecimals(id0)
                            const d1 =
                              id1 === fromToken
                                ? offerDecimals
                                : id1 === toToken
                                  ? receiveDecimals
                                  : registrySwapDecimals(id1)
                            if (d0 == null || d1 == null) return '—'
                            return `${formatTokenAmount(a0.amount, d0)} / ${formatTokenAmount(a1.amount, d1)}`
                          })()}
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
                            commissionAmount && receiveAssetInfo && receiveDecimals != null
                              ? formatTokenAmount(commissionAmount, receiveDecimals)
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
                        <FeeDiscountUnregisteredCta
                          testId="swap-fee-discount-unregistered-cta"
                          className="col-span-2"
                        />
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

            {primaryNotice === 'price' && (
              <SwapImpactAside
                economicExplanation={simData?.economicExplanation}
                routeSlippagePct={expectedSlippagePct}
                blocked={routeSlippageBlocked ? (theaterRouteQuote ? 'extreme' : 'expert') : undefined}
                confirming={showImpactConfirm && isWalletConnected}
                onCancel={() => setShowImpactConfirm(false)}
                onEnableExpert={() => setShowExpertModeModal(true)}
              />
            )}
            {primaryNotice === 'config' && (
              <p className="alert-error mb-3 text-xs" role="alert">
                Wrapping is temporarily unavailable.
              </p>
            )}
            {primaryNotice === 'hop' && (
              <p className="alert-error mb-3 text-xs" role="alert">
                Quote exceeds your price protection.
              </p>
            )}
            {primaryNotice === 'paused' && (
              <div className="alert-error mb-3 text-xs space-y-2" role="alert" data-testid="swap-pair-paused-banner">
                <p>
                  {isPairPaused ? 'Pair paused.' : 'Wrapping is paused.'}{' '}
                  <a className="underline" href={USER_INCIDENT_FAQ_HREF} target="_blank" rel="noopener noreferrer">
                    Docs
                  </a>
                </p>
              </div>
            )}
            {primaryNotice === 'frozen' && (
              <p className="alert-error mb-3 text-xs" role="alert" data-testid="swap-pair-code-id-frozen-banner">
                Trading is temporarily unavailable.
              </p>
            )}
            {primaryNotice === 'restricted' && tradingBlacklist.message && (
              <div className="alert-error mb-3 text-xs" role="alert">
                <p>{tradingBlacklist.message}</p>
              </div>
            )}
            {primaryNotice === 'rate' && (
              <div className="alert-error mb-3 text-xs" role="alert" data-testid="swap-wrap-rate-limit-banner">
                <p>{WRAP_RATE_LIMIT_EXCEEDED_MESSAGE}</p>
              </div>
            )}
            {simData && executableOutput && hasPositiveInputAmount && fromToken && toToken && (
              <details className="mb-3" data-testid="swap-pre-submit-details">
                <summary
                  className="cursor-pointer uppercase text-xs tracking-wide font-medium select-none mb-2"
                  style={{ color: 'var(--ink-subtle)' }}
                >
                  Signing details
                </summary>
                {expectedSlippagePct != null && expectedSlippagePct <= SWAP_FUNDED_HIGH_IMPACT_PCT && (
                  <SwapImpactAside
                    details
                    routeSlippagePct={expectedSlippagePct}
                    economicExplanation={simData.economicExplanation}
                    testId="swap-price-details"
                  />
                )}
                {showFeeDiscountRegistryWarning && (
                  <FeeDiscountRegistryWarning testId="swap-fee-discount-registry-warning" />
                )}
                <SwapPreSubmitSummary
                  offerSymbol={getTokenDisplaySymbol(fromToken)}
                  receiveSymbol={getTokenDisplaySymbol(toToken)}
                  offerAmountHuman={offerDecimals != null ? fromRawAmount(rawInputAmount, offerDecimals) : '—'}
                  receiveAmountHuman={
                    theaterRouteQuote
                      ? '—'
                      : outputAmount && receiveAssetInfo && receiveDecimals != null
                        ? fromRawAmount(outputAmount, receiveDecimals)
                        : '—'
                  }
                  maxSpreadPercent={slippageTolerance}
                  minReceiveHuman={
                    !showQuoteOnly && minReceived != null && receiveAssetInfo && receiveDecimals != null
                      ? fromRawAmount(minReceived, receiveDecimals)
                      : null
                  }
                  pairContractAddresses={swapBlacklistProbe.pairAddresses}
                  chainFullLabel={getNetworkBadgeCopy().fullLabel}
                />
              </details>
            )}
            {/* Swap Button */}
            {primaryNotice === 'acquire' && (
              <SwapPayAcquireGuidanceBanner
                guidance={payAcquireGuidance}
                testIdPrefix="swap"
                hideReduce={acquireReduceHiddenForBroadcastPhase(swapMutation.phase)}
                onReduce={(human) => {
                  setExactField('input')
                  setInputAmount(human)
                  setShowImpactConfirm(false)
                }}
              />
            )}
            <button
              onClick={() => {
                sounds.playButtonPress()
                if (!isWalletConnected) {
                  openWalletModal()
                  return
                }
                if (extraDebitGate.blockSubmit) return
                if (
                  priceImpact &&
                  (expectedSlippagePct ?? 0) > SWAP_FUNDED_HIGH_IMPACT_PCT &&
                  (!showImpactConfirm || impactReviewedQuoteRef.current !== impactReviewKey)
                ) {
                  impactAcceptedQuoteRef.current = null
                  impactReviewedQuoteRef.current = impactReviewKey
                  setShowImpactConfirm(true)
                  return
                }
                impactAcceptedQuoteRef.current = impactReviewKey
                setShowImpactConfirm(false)
                swapMutation.mutate()
              }}
              disabled={buttonDisabled}
              data-testid="swap-submit"
              className={`w-full py-3.5 sm:py-4 font-semibold text-base ${
                buttonDisabled
                  ? 'btn-disabled !w-full !py-3.5 sm:!py-4'
                  : 'btn-primary btn-cta !w-full !py-3.5 sm:!py-4'
              }`}
            >
              {buttonText}
            </button>

            <TerraBroadcastPendingLink
              phase={swapMutation.phase}
              txHash={swapMutation.pendingTxHash}
              className="mt-2 text-[10px] font-mono break-all"
            />

            {primaryNotice === 'tx' && swapMutation.isError && (
              <div className="mt-4" role="alert" data-testid="swap-tx-error">
                <TxResultAlert type="error" message={humanizeUserFacingErrorFromUnknown(swapMutation.error)} />
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
    </LocalQuoteTokenMetadata.Provider>
  )
}
