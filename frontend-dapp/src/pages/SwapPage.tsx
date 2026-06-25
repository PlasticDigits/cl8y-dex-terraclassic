import { useState, useEffect, useCallback, useMemo, useId } from 'react'
import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'
import { assertSubmitHybridAligned, assertSubmitQuotePayRawAligned, SIM_QUOTE_DEBOUNCE_MS } from '@/utils/quoteDebounce'
import { useSubmitAlignedSimQuote } from '@/hooks/useSubmitAlignedSimQuote'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { useWalletStore } from '@/hooks/useWallet'
import { useDexStore } from '@/stores/dex'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { simulateSwap, swap, getPool } from '@/services/terraclassic/pair'
import { quoteDirectHybridSwap } from '@/utils/directHybridQuote'
import {
  computeDirectHybridMinReturn,
  enrichSwapOperationsWithHopMinReturns,
  preflightSwapRouteSpread,
  type SwapRoutePreflightSpread,
} from '@/services/terraclassic/swapRoutePreflight'
import { getPairFeeConfig } from '@/services/terraclassic/settings'
import { getTokenBalance } from '@/services/terraclassic/queries'
import { getTraderDiscount, getRegistration } from '@/services/terraclassic/feeDiscount'
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
} from '@/services/terraclassic/router'
import { hybridParamsWithSubmitCap } from '@/services/terraclassic/hybridSwapGas'
import { netUlunaAfterTransferTaxAsync } from '@/utils/nativeTransferTax'
import { hybridFromSingleHopIndexerOps, swapOpsRequireRouter } from '@/services/terraclassic/swapRouting'
import { queryPausedState, checkRateLimitExceeded } from '@/services/terraclassic/wrapMapper'
import { DOCS_GITLAB_BASE, FEE_DISCOUNT_CONTRACT_ADDRESS, WRAP_MAPPER_CONTRACT_ADDRESS } from '@/utils/constants'
import {
  assetInfoLabel,
  tokenAssetInfo,
  isNativeDenom,
  type HybridSwapParams,
  type IndexerRouteSolveResponse,
  type IndexerRouteQuoteKind,
} from '@/types'
import { sounds } from '@/lib/sounds'
import { FeeDisplay, TxResultAlert, TokenSelect, Spinner, RetryError } from '@/components/ui'
import { TerraBroadcastPendingLink } from '@/components/ui/TerraBroadcastPendingLink'
import { terraBroadcastPendingButtonLabel } from '@/utils/terraBroadcastUi'
import { LcdQueryGate } from '@/components/common/LcdQueryGate'
import { MarketDataServiceOutageBanner } from '@/components/common/MarketDataServiceOutageBanner'
import { fetchCW20TokenInfo, getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { formatTokenAmount, getDecimals, toRawAmount } from '@/utils/formatAmount'
import { isPositiveDecimalAmount } from '@/utils/decimalAmountInput'
import { spreadPercentFromRawSim } from '@/utils/rawAmountMath'
import { computeMaxSpendableHumanAmount } from '@/utils/maxSpendableAmount'
import { AmountBalanceActions } from '@/components/common/AmountBalanceActions'
import { getFeeDiscountHealth, getRouteSolve } from '@/services/indexer/client'
import { swapOperationsFromIndexerResponse } from '@/services/indexer/routeOperations'
import { getDirectHybridBookSplit, getIndexerHybridExecutionSummary } from '@/utils/swapDisclosure'
import {
  computeSwapRouteDisplay,
  deriveSwapSubmitRouteSource,
  SWAP_CLIENT_BFS_FALLBACK_COPY,
} from '@/utils/swapRouteDisplay'
import { humanizeUserFacingError, humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'
import { isIndexerPairNotFoundError, isIndexerUnavailableError } from '@/utils/indexerErrors'
import { MARKET_DATA_SERVICE_OUTAGE_TITLE, SWAP_MARKET_DATA_OUTAGE_LEAD } from '@/utils/marketDataServiceCopy'
import { detectSwapIndexerOutage } from '@/utils/swapIndexerOutage'
import {
  FEE_DISCOUNT_REGISTRY_WARNING_TEXT,
  resolveFeeDiscountRegistryStatus,
  shouldShowFeeDiscountRegistryWarning,
} from '@/utils/feeDiscountRegistryWarning'
import { useQueryManualRetry } from '@/hooks/useQueryManualRetry'
import { useTradingBlacklist } from '@/hooks/useTradingBlacklist'
import { usePairPaused } from '@/hooks/usePairPaused'
import { USER_INCIDENT_FAQ_HREF } from '@/components/legal/legalCopy'
import { TradeOnboardingStrip } from '@/components/common/TradeOnboardingStrip'
import { ExpertModeModal } from '@/components/swap/ExpertModeModal'
import { SwapAdvancedSettings } from '@/components/swap/SwapAdvancedSettings'
import { SwapPreSubmitSummary } from '@/components/swap/SwapPreSubmitSummary'
import { readSwapSettingsAdvancedOpen, writeSwapSettingsAdvancedOpen } from '@/utils/swapSettingsAdvanced'
import { getNetworkBadgeCopy } from '@/utils/networkDisplay'
import {
  SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT,
  SWAP_EXTREME_SLIPPAGE_WARNING_PCT,
  parseSlippagePercent,
  resolveRouteSlippagePercent,
  resolveSwapExpectedSlippagePercent,
  slippageSeverityClass,
} from '@/utils/swapRouteSlippage'
import {
  formatTransactionDeadline,
  ROUTE_EXECUTION_SLIPPAGE_LABEL,
  ROUTE_EXECUTION_SLIPPAGE_TOOLTIP,
  SLIPPAGE_PROTECTION_LABEL,
  SLIPPAGE_PROTECTION_ON_CHAIN_FOOTNOTE,
  TRANSACTION_DEADLINE_LABEL,
} from '@/utils/slippageProtectionCopy'
/** Wallet-side simulation result with optional indexer-routing metadata. */
interface SwapSimData {
  return_amount: string
  spread_amount: string
  commission_amount: string
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

  const [inputAmount, setInputAmount] = useState('')
  const [fromToken, setFromToken] = useState<string>('')
  const [toToken, setToToken] = useState<string>('')
  const [showSettings, setShowSettings] = useState(false)
  const [showAdvancedSettings, setShowAdvancedSettings] = useState(readSwapSettingsAdvancedOpen)
  const [showExpertModeModal, setShowExpertModeModal] = useState(false)
  const [customSlippage, setCustomSlippage] = useState('')
  const [customDeadlineMinutes, setCustomDeadlineMinutes] = useState('')
  const [showImpactConfirm, setShowImpactConfirm] = useState(false)
  const [indexerRouteResult, setIndexerRouteResult] = useState<IndexerRouteSolveResponse | null>(null)
  const [indexerRouteError, setIndexerRouteError] = useState<string | null>(null)
  const [indexerRouteLoading, setIndexerRouteLoading] = useState(false)
  const [useHybridBook, setUseHybridBook] = useState(true)
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

  useEffect(() => {
    if (pairs.length > 0 && !fromToken) {
      const tokens = getAllTokens(pairs)
      if (tokens.length >= 2) {
        setFromToken(tokens[0])
        setToToken(tokens[1])
      }
    }
  }, [pairs, fromToken])

  const allTokens = useMemo(() => (pairs.length > 0 ? getAllTokens(pairs) : []), [pairs])

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
    if (!isDirect || !useHybridBook || !fromToken.startsWith('terra1')) return false
    const t = bookInputHuman.trim()
    if (!t) return false
    return isPositiveDecimalAmount(t)
  }, [isDirect, useHybridBook, fromToken, bookInputHuman])

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

  const discountQuery = useQuery({
    queryKey: ['traderDiscount', address],
    queryFn: () => {
      if (!address) throw new Error('No address')
      return getTraderDiscount(address)
    },
    enabled: !!address && !!FEE_DISCOUNT_CONTRACT_ADDRESS,
    staleTime: 15_000,
  })

  const registrationQuery = useQuery({
    queryKey: ['feeDiscountRegistration', address],
    queryFn: () => {
      if (!address) throw new Error('No address')
      return getRegistration(address)
    },
    enabled: !!address && !!FEE_DISCOUNT_CONTRACT_ADDRESS,
    staleTime: 15_000,
  })

  const feeDiscountHealthQuery = useQuery({
    queryKey: ['feeDiscountHealth'],
    queryFn: getFeeDiscountHealth,
    enabled: !!FEE_DISCOUNT_CONTRACT_ADDRESS,
    staleTime: 30_000,
    refetchInterval: 30_000,
    retry: false,
  })

  const feeDiscountRegistryInput = useMemo(
    () => ({
      feeDiscountContractConfigured: !!FEE_DISCOUNT_CONTRACT_ADDRESS,
      registration: registrationQuery.data,
      discount: discountQuery.data,
      registrationQueryError: registrationQuery.isError,
      discountQueryError: discountQuery.isError,
      indexerHealth: feeDiscountHealthQuery.isSuccess ? feeDiscountHealthQuery.data : null,
    }),
    [
      registrationQuery.data,
      discountQuery.data,
      registrationQuery.isError,
      discountQuery.isError,
      feeDiscountHealthQuery.isSuccess,
      feeDiscountHealthQuery.data,
    ]
  )

  const feeDiscountRegistryStatus = useMemo(
    () => resolveFeeDiscountRegistryStatus(feeDiscountRegistryInput),
    [feeDiscountRegistryInput]
  )

  const showFeeDiscountRegistryWarning = shouldShowFeeDiscountRegistryWarning(feeDiscountRegistryInput)

  const balanceQuery = useQuery({
    queryKey: ['tokenBalance', address, fromToken],
    queryFn: () => {
      if (!address || !offerAssetInfo) throw new Error('Missing params')
      return getTokenBalance(address, offerAssetInfo)
    },
    enabled: !!address && !!offerAssetInfo,
    refetchInterval: 15_000,
  })

  const offerDecimals = offerAssetInfo ? getDecimals(offerAssetInfo) : 6
  const rawInputAmount = inputAmount ? toRawAmount(inputAmount, offerDecimals) : '0'
  const debouncedInputAmount = useDebouncedValue(inputAmount, SIM_QUOTE_DEBOUNCE_MS)
  const debouncedRawInputAmount = debouncedInputAmount ? toRawAmount(debouncedInputAmount, offerDecimals) : '0'

  const needsWrapCheck = isWrapOrUnwrap ? wrapUnwrapType === 'wrap' : (nativeRouteInfo?.needsWrapInput ?? false)
  const wrapDenom = needsWrapCheck ? (isNativeDenom(fromToken) ? fromToken : null) : null

  const payIsNativeUluna = isNativeDenom(fromToken)
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
            needsWrapInput: nativeRouteInfo?.needsWrapInput ?? false,
            needsUnwrapOutput: nativeRouteInfo?.needsUnwrapOutput ?? false,
            hopCount: nativeRouteInfo?.operations?.length,
          }
        : undefined,
    })
  }, [
    balanceQuery.data,
    offerDecimals,
    payIsNativeUluna,
    wrapUnwrapType,
    nativeRouteInfo?.needsWrapInput,
    nativeRouteInfo?.needsUnwrapOutput,
    nativeRouteInfo?.operations?.length,
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

  const pausedQuery = useQuery({
    queryKey: ['wrapMapperPaused'],
    queryFn: queryPausedState,
    enabled:
      !!WRAP_MAPPER_CONTRACT_ADDRESS &&
      (needsWrapCheck ||
        (isWrapOrUnwrap && wrapUnwrapType === 'unwrap') ||
        (nativeRouteInfo?.needsUnwrapOutput ?? false)),
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

  const isWrapPaused = pausedQuery.data === true
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
        useHybridBook,
        debouncedBookInputHuman,
        debouncedHybridMaxMakers,
        slippageTolerance,
        address,
      ] as const,
    [
      fromToken,
      toToken,
      debouncedRawInputAmount,
      route,
      wrapUnwrapType,
      nativeRouteInfo,
      useHybridBook,
      debouncedBookInputHuman,
      debouncedHybridMaxMakers,
      slippageTolerance,
      address,
    ]
  )

  const quoteTrader = useMemo(() => (address ? { trader: address } : undefined), [address])

  const simQuery = useQuery({
    queryKey: simQueryKey,
    placeholderData: keepPreviousData,
    queryFn: async (): Promise<SwapSimData> => {
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
        return {
          return_amount: simRaw,
          spread_amount: '0',
          commission_amount: '0',
        }
      }

      if (nativeRouteInfo) {
        const result = await simulateNativeSwap(simRaw, fromToken, toToken, pairs)
        let routePreflight: SwapRoutePreflightSpread | undefined
        if (nativeRouteInfo.operations.length > 0) {
          let preflightOffer = simRaw
          if (nativeRouteInfo.needsWrapInput) {
            preflightOffer = (await netUlunaAfterTransferTaxAsync(BigInt(simRaw), fromToken)).toString()
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
          routePreflight,
        })
      }

      // Advanced: manual limit-book split on a direct pair (overrides indexer hybrid for this quote).
      if (isDirect && directPair && useHybridBook && fromToken.startsWith('terra1')) {
        const hybridSplit = getDirectHybridBookSplit({
          isDirect: true,
          useHybridBook,
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

      // Default CW20↔CW20: indexer hybrid optimization (≤3 hops) + wallet `simulate_swap_operations`.
      if (fromToken.startsWith('terra1') && toToken.startsWith('terra1') && simRaw !== '0') {
        try {
          const idx = await getRouteSolve(fromToken, toToken, simRaw, {
            maxMakerFills: debouncedHybridMaxMakers,
            trader: quoteTrader?.trader,
          })
          const tin = idx.token_in.trim().toLowerCase()
          const tout = idx.token_out.trim().toLowerCase()
          if (tin === fromToken.trim().toLowerCase() && tout === toToken.trim().toLowerCase()) {
            const ops = swapOperationsFromIndexerResponse(idx.router_operations as unknown[], idx.hops.length)
            const opsForQuote = await enrichSwapOperationsWithHopMinReturns(ops, simRaw, slippageTolerance, quoteTrader)
            const result = await simulateMultiHopSwap(simRaw, opsForQuote, quoteTrader)
            const routePreflight = await preflightSwapRouteSpread(opsForQuote, simRaw, maxSpreadStr, quoteTrader)
            const intermediates =
              idx.intermediate_tokens?.length === idx.hops.length + 1
                ? idx.intermediate_tokens
                : [idx.token_in, ...idx.hops.map((h) => h.ask_token)]
            return {
              return_amount: result.amount,
              spread_amount: '0',
              commission_amount: '0',
              routeSlippagePercent: resolveRouteSlippagePercent(
                result.amount,
                idx.spot_amount_out,
                idx.slippage_percent
              ),
              spotAmountOut: idx.spot_amount_out,
              indexerQuoteKind: idx.quote_kind,
              indexerOperations: opsForQuote,
              indexerIntermediateTokens: intermediates,
              routePreflight,
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
          useHybridBook,
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
          return withIndexerOutageFlag({
            ...quoted,
            routeSlippagePercent: quoted.routeSlippagePercent,
            spotAmountOut: quoted.spotAmountOut,
          })
        }
        const offerInfo = tokenAssetInfo(fromToken)
        const r = await simulateSwap(directPair.contract_addr, offerInfo, simRaw, quoteTrader)
        return withIndexerOutageFlag(r)
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
    enabled: hasRoute && isPositiveDecimalAmount(debouncedInputAmount),
    refetchInterval: 10_000,
  })

  const swapBlacklistProbe = useMemo(() => {
    const routeOps = route ?? nativeRouteInfo?.operations ?? simQuery.data?.indexerOperations
    const tokens = new Set<string>()
    if (fromToken?.startsWith('terra1')) tokens.add(fromToken)
    if (toToken?.startsWith('terra1')) tokens.add(toToken)

    const addresses = new Set<string>()
    if (routeOps && routeOps.length > 0) {
      for (const op of routeOps) {
        const offer = assetInfoLabel(op.terra_swap.offer_asset_info)
        const ask = assetInfoLabel(op.terra_swap.ask_asset_info)
        if (offer.startsWith('terra1')) tokens.add(offer)
        if (ask.startsWith('terra1')) tokens.add(ask)
        const matched = pairs.find((p) => {
          const a = assetInfoLabel(p.asset_infos[0])
          const b = assetInfoLabel(p.asset_infos[1])
          return (a === offer && b === ask) || (b === offer && a === ask)
        })
        if (matched?.contract_addr.startsWith('terra1')) {
          addresses.add(matched.contract_addr)
        }
      }
    } else if (directPair?.contract_addr.startsWith('terra1')) {
      addresses.add(directPair.contract_addr)
    }
    const pairAddresses = [...addresses]
    return { tokens: [...tokens], pairAddresses }
  }, [route, nativeRouteInfo, simQuery.data?.indexerOperations, directPair, pairs, fromToken, toToken])

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

  const simRetry = useQueryManualRetry(simQueryKey, simQuery)

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
      useHybridBook,
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
  }, [isDirect, useHybridBook, fromToken, debouncedBookInputHuman, debouncedRawInputAmount, debouncedHybridMaxMakers])

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
    hybrid: useHybridBook
      ? {
          enabled: true,
          live: hybridStaleLive,
          snapshotted: hybridSubmitSnapshot,
        }
      : undefined,
  })

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
      const submitMinReceived = minReceived
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
        useHybridBook,
        fromToken,
        bookInputHuman: debouncedBookInputHuman,
        rawInputAmount: debouncedRawInputAmount,
        hybridMaxMakers: debouncedHybridMaxMakers,
      }),
    [isDirect, useHybridBook, fromToken, debouncedBookInputHuman, debouncedRawInputAmount, debouncedHybridMaxMakers]
  )

  const indexerHybridExec = useMemo(
    () => getIndexerHybridExecutionSummary(simData?.indexerQuoteKind),
    [simData?.indexerQuoteKind]
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

  const insufficientBalance =
    hasPositiveInputAmount && balanceQuery.data !== undefined && BigInt(rawInputAmount) > BigInt(balanceQuery.data)

  let buttonText = 'Swap'
  let buttonDisabled = false
  if (!isWalletConnected) {
    buttonText = 'Connect Wallet'
    buttonDisabled = false
  } else if (!hasRoute) {
    buttonText = 'No Route'
    buttonDisabled = true
  } else if (isWrapPaused) {
    buttonText = 'Wrapping is Temporarily Paused'
    buttonDisabled = true
  } else if (isPairPaused) {
    buttonText = 'Pair is paused'
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
    buttonText = 'Calculating...'
    buttonDisabled = true
  } else if (swapMutation.isPending) {
    buttonText = terraBroadcastPendingButtonLabel(swapMutation.phase, true, 'Swap', 'Swapping…')
    buttonDisabled = true
  } else if (showImpactConfirm) {
    buttonText = `Confirm Swap (${priceImpact}% impact)`
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
      // Block non-numeric input: only allow digits and one decimal point
      const sanitized = value.replace(/[^\d.]/g, '').replace(/(\.\d*)\./g, '$1') // keep only first decimal (e.g. "5.5.5" -> "5.55")
      setCustomSlippage(sanitized)
      const parsed = parseFloat(sanitized)
      if (!isNaN(parsed) && parsed >= 0.01 && parsed <= 50) {
        setSlippageTolerance(parsed)
      } else if (!isNaN(parsed) && parsed > 50) {
        setSlippageTolerance(50)
      }
    },
    [setSlippageTolerance]
  )

  const customSlippageError =
    customSlippage !== '' &&
    (isNaN(parseFloat(customSlippage)) || parseFloat(customSlippage) < 0.01 || parseFloat(customSlippage) > 50)

  const handleToggleSettings = useCallback(() => {
    sounds.playButtonPress()
    setShowSettings((prev) => !prev)
  }, [])

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
    <div className="relative max-w-[620px] mx-auto w-full">
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
            <button
              onClick={handleToggleSettings}
              className="btn-muted !text-[11px] sm:!text-xs !px-2.5 sm:!px-3 !py-1"
            >
              Settings
            </button>
          </div>

          <TradeOnboardingStrip />

          {indexerOutage && (
            <MarketDataServiceOutageBanner
              testId="swap-market-data-outage-banner"
              title={MARKET_DATA_SERVICE_OUTAGE_TITLE}
              lead={SWAP_MARKET_DATA_OUTAGE_LEAD}
              onRetry={simRetry.retry}
            />
          )}

          {showFeeDiscountRegistryWarning && (
            <div className="alert-warning text-sm mb-4" role="status" data-testid="swap-fee-discount-registry-warning">
              {FEE_DISCOUNT_REGISTRY_WARNING_TEXT}
            </div>
          )}

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
                <p className="label-glass mb-1">{SLIPPAGE_PROTECTION_LABEL}</p>
                <p className="mb-3 text-[10px] leading-relaxed" style={{ color: 'var(--ink-subtle)' }}>
                  {SLIPPAGE_PROTECTION_ON_CHAIN_FOOTNOTE}
                </p>
                <div className="flex flex-wrap gap-2">
                  {[0.1, 0.5, 1.0].map((val) => (
                    <button
                      key={val}
                      onClick={() => handleSlippagePreset(val)}
                      className={`tab-glass !text-xs !px-3 !py-1.5 ${
                        slippageTolerance === val && !customSlippage ? 'tab-glass-active' : 'tab-glass-inactive'
                      }`}
                    >
                      {val}%
                    </button>
                  ))}
                  <div className="relative flex-1">
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
                </div>
                {customSlippageError && (
                  <p
                    className="mt-2 text-xs font-semibold uppercase tracking-wide"
                    style={{ color: 'var(--color-negative)' }}
                  >
                    Must be between 0.01% and 50%
                  </p>
                )}
                {!customSlippageError && slippageTolerance > 5 && (
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
                  <p className="mt-2 text-[10px] leading-relaxed" style={{ color: 'var(--ink-subtle)' }}>
                    Swap transactions expire if not included on-chain within this window (default 5 min).
                  </p>
                </div>
                <div className="mt-4 pt-3 border-t" style={{ borderColor: 'var(--border-subtle)' }}>
                  <label className="flex items-center gap-2 text-xs cursor-pointer">
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
                  <p className="mt-1 text-[10px] leading-relaxed" style={{ color: 'var(--ink-subtle)' }}>
                    When off, swaps with expected slippage above {SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT}% (vs best-route
                    token prices) are blocked.
                  </p>
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
                useHybridBook={useHybridBook}
                onUseHybridBookChange={setUseHybridBook}
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
                <TokenSelect
                  value={fromToken}
                  tokens={allTokens}
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
                value={inputAmount}
                onChange={(e) => {
                  const v = e.target.value
                  if (v === '' || /^\d*\.?\d*$/.test(v)) {
                    setInputAmount(v)
                    setShowImpactConfirm(false)
                  }
                }}
                placeholder="0.00"
                className="swap-io-amount-input w-full text-[1.75rem] sm:text-2xl font-medium bg-transparent"
                style={{ color: 'var(--ink)' }}
              />
              {isWalletConnected && (
                <AmountBalanceActions
                  balanceQuery={balanceQuery}
                  decimals={offerDecimals}
                  walletConnected={isWalletConnected}
                  showBalance={!!offerAssetInfo}
                  spendableRaw={payMaxResult.spendableRaw}
                  onMax={() => setInputAmount(payMaxResult.human)}
                  onFraction={(human) => {
                    setInputAmount(human)
                    setShowImpactConfirm(false)
                  }}
                  testIdMax="swap-pay-max"
                  testIdFractionPrefix="swap-pay-frac"
                />
              )}
            </div>

            <div className="relative z-20 flex justify-center pointer-events-none -my-[22px] sm:-my-6">
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
                className="pointer-events-auto w-11 h-11 sm:w-12 sm:h-12 rounded-[18px] border flex items-center justify-center transition-all hover:-translate-y-0.5"
                style={{
                  borderColor: 'rgba(255, 225, 190, 0.2)',
                  background:
                    'linear-gradient(180deg, rgba(72, 44, 31, 0.98), rgba(37, 22, 18, 0.99)), rgba(255, 255, 255, 0.03)',
                  color: 'var(--cyan)',
                  boxShadow:
                    '0 16px 34px rgba(0, 0, 0, 0.24), 0 0 0 1px rgba(255, 161, 59, 0.08), inset 0 1px 0 rgba(255, 243, 221, 0.2)',
                }}
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

            <div className="card-glass swap-io-card-receive !p-4 sm:!p-5">
              <div className="flex flex-col gap-2 mb-3 sm:flex-row sm:items-start sm:justify-between sm:gap-3">
                <span className="label-glass !mb-0 sm:pt-1">You Receive</span>
                <TokenSelect
                  value={toToken}
                  tokens={allTokens}
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
              <div className="text-[1.75rem] sm:text-2xl font-medium" style={{ color: 'var(--ink)' }}>
                {simQuery.isFetching ? (
                  <span className="animate-pulse" style={{ color: 'var(--ink-subtle)' }}>
                    Calculating...
                  </span>
                ) : simData && outputAmount && receiveAssetInfo ? (
                  formatTokenAmount(outputAmount, getDecimals(receiveAssetInfo))
                ) : (
                  <span style={{ color: 'var(--ink-subtle)' }}>0.00</span>
                )}
              </div>
            </div>
          </div>

          <div className="mb-4 space-y-2">
            {simData && (indexerHybridExec.show || directHybridBookSplit) && (
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
                {directHybridBookSplit && (
                  <div>
                    <p className="uppercase tracking-wide font-semibold mb-0.5" style={{ color: 'var(--ink-subtle)' }}>
                      {indexerHybridExec.show ? 'Settings — direct pay split' : 'Execution'}
                    </p>
                    {directHybridBookSplit.bookExceedsPay && (
                      <p className="font-medium" style={{ color: 'var(--color-negative)' }}>
                        Book leg is larger than your pay amount.
                      </p>
                    )}
                    {!directHybridBookSplit.bookExceedsPay && directHybridBookSplit.willSubmitHybrid && (
                      <>
                        <p style={{ color: 'var(--ink)' }}>
                          <span className="font-semibold">Hybrid (pool + limit book)</span> — pool{' '}
                          {directHybridBookSplit.poolHuman} · book {directHybridBookSplit.bookHuman}{' '}
                          {getTokenDisplaySymbol(fromToken)}
                        </p>
                        <p className="text-[10px] font-mono" style={{ color: 'var(--ink-subtle)' }}>
                          {directHybridBookSplit.poolRaw} (pool raw) · {directHybridBookSplit.bookRaw} (book raw)
                        </p>
                      </>
                    )}
                    {!directHybridBookSplit.bookExceedsPay && !directHybridBookSplit.willSubmitHybrid && (
                      <>
                        {BigInt(directHybridBookSplit.bookRaw) > 0n ? (
                          <p style={{ color: 'var(--color-warning, #f59e0b)' }}>
                            Book leg is set, but the hybrid path will not be submitted. Set{' '}
                            <strong>max distinct makers</strong> to at least 1.
                          </p>
                        ) : (
                          <p style={{ color: 'var(--ink)' }}>
                            <span className="font-semibold">Pool only</span> — add a book leg in Settings to use the
                            on-chain book for part of the pay.
                          </p>
                        )}
                      </>
                    )}
                  </div>
                )}
              </div>
            )}
            {fromToken && toToken && !hasRoute && (
              <div className="alert-error !text-xs">No route found between these tokens</div>
            )}
          </div>

          {/* Trade Details */}
          {simData && (
            <div className="card-glass mb-4 grid grid-cols-2 gap-x-3 gap-y-2 text-xs sm:text-sm sm:block sm:space-y-2">
              {poolQuery.data && (
                <div
                  className="min-w-0 flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between"
                  style={{ color: 'var(--ink-dim)' }}
                >
                  <span className="uppercase text-xs tracking-wide font-medium">Pool Reserves</span>
                  <span className="font-mono text-xs sm:text-right break-all">
                    {formatTokenAmount(poolQuery.data.assets[0].amount, getDecimals(poolQuery.data.assets[0].info))} /{' '}
                    {formatTokenAmount(poolQuery.data.assets[1].amount, getDecimals(poolQuery.data.assets[1].info))}
                  </span>
                </div>
              )}
              {feeQuery.data && (
                <div
                  className="min-w-0 flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between"
                  style={{ color: 'var(--ink-dim)' }}
                >
                  <span className="uppercase text-xs tracking-wide font-medium">Fee</span>
                  <FeeDisplay
                    feeBps={feeQuery.data.fee_bps}
                    discountBps={discountQuery.data?.discount_bps}
                    commissionAmount={
                      commissionAmount && receiveAssetInfo
                        ? formatTokenAmount(commissionAmount, getDecimals(receiveAssetInfo))
                        : undefined
                    }
                  />
                </div>
              )}
              {address && FEE_DISCOUNT_CONTRACT_ADDRESS && feeDiscountRegistryStatus === 'unregistered' && (
                <div
                  className="col-span-2 p-2 border-2 rounded-none text-xs shadow-[1px_1px_0_#000]"
                  style={{
                    borderColor: 'color-mix(in srgb, var(--cyan) 30%, transparent)',
                    background: 'color-mix(in srgb, var(--cyan) 5%, transparent)',
                    color: 'var(--cyan)',
                  }}
                >
                  <a href="/tiers" className="hover:underline uppercase tracking-wide font-semibold">
                    Hold CL8Y to reduce swap fees &rarr;
                  </a>
                </div>
              )}
              {swapRouteLine && (
                <div
                  data-testid="swap-route-summary"
                  className="min-w-0 flex flex-col gap-1 sm:flex-row sm:items-start sm:justify-between col-span-2"
                  style={{ color: 'var(--ink-dim)' }}
                >
                  <span className="uppercase text-xs tracking-wide font-medium shrink-0">Route</span>
                  <div className="font-mono text-xs sm:text-right break-words min-w-0">
                    <span>{swapRouteLine}</span>
                    {isWrapOrUnwrap && (
                      <span className="block mt-0.5 text-[10px] font-sans" style={{ color: 'var(--ink-subtle)' }}>
                        {wrapUnwrapType === 'wrap' ? 'Wrap (1:1)' : 'Unwrap (1:1)'}
                      </span>
                    )}
                    {nativeRouteInfo && (nativeRouteInfo.needsWrapInput || nativeRouteInfo.needsUnwrapOutput) && (
                      <span
                        className="block mt-0.5 text-[10px] font-sans leading-snug"
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
                        className="block mt-0.5 text-[10px] font-sans leading-snug"
                        style={{ color: 'var(--color-warning, #f59e0b)' }}
                      >
                        {SWAP_CLIENT_BFS_FALLBACK_COPY}
                      </span>
                    )}
                  </div>
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
                      vs best-route token prices (indexer). Hop spread: {hopSpreadPercent ?? '—'}%.
                    </p>
                  )}
                  {(parseSlippagePercent(priceImpact) ?? 0) > 5 && (
                    <div className="col-span-2 alert-error !text-xs">
                      High expected slippage! The quoted route deviates significantly from fair cross-rate token prices.
                    </div>
                  )}
                  {extremeSlippageWarning && (
                    <div
                      className="col-span-2 alert-error !text-xs"
                      data-testid="swap-extreme-slippage-warning"
                      role="alert"
                    >
                      <p className="font-semibold mb-1">Extreme slippage ({priceImpact}%)</p>
                      <p>
                        This quote is far from the fair cross-rate. It may exploit mispriced pools or show a
                        misleadingly good multi-hop path. Proceed only if you understand the risk.
                      </p>
                    </div>
                  )}
                </>
              )}
              {minReceived !== null && (
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
          )}

          {simData?.routePreflight && (
            <div className="card-glass mb-3 text-[11px] sm:text-xs leading-relaxed" style={{ color: 'var(--ink-dim)' }}>
              <span className="uppercase tracking-wide font-semibold" style={{ color: 'var(--ink-subtle)' }}>
                Route spread check:{' '}
              </span>
              Worst hop ≈ {simData.routePreflight.worstSpreadPercent}% of gross on that hop (pair simulation, matches
              on-chain max spread logic). See{' '}
              <a
                href="https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs/swap-max-spread-ux.md"
                target="_blank"
                rel="noopener noreferrer"
                className="underline font-mono text-[10px]"
              >
                docs/swap-max-spread-ux.md
              </a>
              .
            </div>
          )}
          {routeSlippageBlocked && (
            <div className="alert-error mb-3 text-xs" role="alert" data-testid="swap-slippage-blocked">
              <p className="font-semibold mb-1">Slippage is too high</p>
              <p className="mb-2">
                Expected slippage is {priceImpact}% (above {SWAP_EXPERT_MODE_SLIPPAGE_BLOCK_PCT}% vs best-route token
                prices). This swap is blocked until you enable Expert Mode.
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
              <p>
                At least one hop in the route has price impact above your slippage protection ({slippageTolerance}% max
                spread). Try a smaller amount, another route, or increase slippage protection in Settings (higher
                protection increases execution risk).
              </p>
            </div>
          )}
          {(showHybridBookSubmitWarning ||
            simData?.indexerQuoteKind === 'indexer_hybrid_lcd' ||
            simData?.indexerQuoteKind === 'indexer_hybrid_lcd_degraded') && (
            <div className="alert-error mb-3 text-xs" role="alert">
              <p className="font-semibold mb-1">Limit book leg</p>
              <p>
                The on-screen estimate may differ from what you receive if market conditions change between quote and
                submit.{' '}
                <a
                  href={`${DOCS_GITLAB_BASE}/limit-orders.md`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="underline"
                >
                  Learn more about limit book routing
                </a>
                .
              </p>
            </div>
          )}
          {isPairPaused && (
            <div className="alert-error mb-3 text-xs space-y-2" role="alert" data-testid="swap-pair-paused-banner">
              <p>
                Pair is paused — swaps are blocked until governance unpauses. Your wallet balances are unchanged; limit
                escrow in the pair contract stays until unpause.
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
          {tradingBlacklist.blocked && tradingBlacklist.message && (
            <div className="alert-error mb-3 text-xs" role="alert">
              <p>{tradingBlacklist.message}</p>
            </div>
          )}
          {simData && hasPositiveInputAmount && fromToken && toToken && (
            <SwapPreSubmitSummary
              offerSymbol={getTokenDisplaySymbol(fromToken)}
              receiveSymbol={getTokenDisplaySymbol(toToken)}
              offerAmountHuman={inputAmount}
              receiveAmountHuman={
                outputAmount && receiveAssetInfo ? formatTokenAmount(outputAmount, getDecimals(receiveAssetInfo)) : '—'
              }
              maxSpreadPercent={slippageTolerance}
              minReceiveHuman={
                minReceived != null && receiveAssetInfo
                  ? formatTokenAmount(minReceived, getDecimals(receiveAssetInfo))
                  : null
              }
              chainFullLabel={getNetworkBadgeCopy().fullLabel}
            />
          )}
          {/* Swap Button */}
          {showImpactConfirm && !routeSlippageBlocked && (
            <div className="alert-error mb-3 text-xs">
              <p className="font-semibold mb-1">High expected slippage warning</p>
              <p>
                This trade has {priceImpact}% expected slippage vs best-route token prices. Click the button again to
                confirm.
              </p>
            </div>
          )}
          <button
            onClick={() => {
              sounds.playButtonPress()
              if (!isWalletConnected) {
                openWalletModal()
                return
              }
              if (priceImpact && (parseSlippagePercent(priceImpact) ?? 0) > 5 && !showImpactConfirm) {
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
