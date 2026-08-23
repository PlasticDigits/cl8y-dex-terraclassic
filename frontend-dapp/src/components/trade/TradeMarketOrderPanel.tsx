import { useMemo, useState, useId, useLayoutEffect } from 'react'
import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'
import {
  assertSubmitHybridAligned,
  assertSubmitQuotePayRawAligned,
  shouldShowSimReceiveCalculating,
  SIM_QUOTE_DEBOUNCE_MS,
  simQuoteRefetchInterval,
} from '@/utils/quoteDebounce'
import { useSubmitAlignedSimQuote } from '@/hooks/useSubmitAlignedSimQuote'
import { useTerraBroadcastMutation } from '@/hooks/useTerraBroadcastMutation'
import { useWalletStore } from '@/hooks/useWallet'
import { useDexStore } from '@/stores/dex'
import { useLimitOrderEscrowBalance } from '@/hooks/useLimitOrderEscrowBalance'
import { useNativeUlunaBalance } from '@/hooks/useNativeUlunaBalance'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { simulateSwap, swap } from '@/services/terraclassic/pair'
import {
  computeDirectHybridMinReturn,
  enrichSwapOperationsWithHopMinReturns,
  type SwapRoutePreflightSpread,
} from '@/services/terraclassic/swapRoutePreflight'
import { executeMultiHopSwap, type SwapOperation } from '@/services/terraclassic/router'
import { hybridParamsWithSubmitCap } from '@/services/terraclassic/hybridSwapGas'
import { hybridFromSingleHopIndexerOps, swapOpsRequireRouter } from '@/services/terraclassic/swapRouting'
import {
  executeCw20AllowanceThen,
  estimateMarketPairSwapSequenceUlunaFeesTotal,
} from '@/services/terraclassic/transactions'
import {
  POOL_ONLY_QUOTE_DISCLOSURE,
  quoteDirectHybridSwap,
  quoteDisclosureForIndexerKind,
  DIRECT_HYBRID_AMOUNT_RECONCILED_COPY,
} from '@/utils/directHybridQuote'
import { quoteCw20ViaRouteSolve } from '@/utils/cw20RouteSolveQuote'
import { humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'
import { DOCS_GITLAB_BASE } from '@/utils/constants'
import { sounds } from '@/lib/sounds'
import { SLIPPAGE_PROTECTION_LABEL } from '@/utils/slippageProtectionCopy'
import { SlippageProtectionPresets } from '@/components/common/SlippageProtectionPresets'
import { Spinner } from '@/components/ui'
import { terraBroadcastPendingButtonLabel } from '@/utils/terraBroadcastUi'
import {
  assetInfoLabel,
  tokenAssetInfo,
  type HybridSwapParams,
  type IndexerRouteQuoteKind,
  type PairInfo,
} from '@/types'
import { getDecimals, toRawAmount, formatTokenAmount } from '@/utils/formatAmount'
import { isDecimalAmountDraft } from '@/utils/decimalAmountInput'
import { computeMaxSpendableHumanAmount } from '@/utils/maxSpendableAmount'
import { AmountBalanceActions } from '@/components/common/AmountBalanceActions'
import { evaluateLimitOrderEscrowPlaceGate } from '@/utils/limitOrderEscrowBalanceGate'
import { evaluateMarketSwapNativeGasPlaceGate } from '@/utils/limitOrderNativeGasBalanceGate'
import { getDirectHybridBookSplit, getIndexerHybridExecutionSummary } from '@/utils/swapDisclosure'
import { LimitOrderEscrowAmountField } from '@/components/trade/LimitOrderEscrowAmountField'
import { useCommunityTaxSellBps } from '@/hooks/useCommunityTaxSellBps'
import {
  communityTaxExecuteUsesRouter,
  communityTaxRouteHint,
  extraDebitSellBpsForExecute,
} from '@/utils/taxPreviewMaxSpend'
import { SwapPreSubmitSummary } from '@/components/swap/SwapPreSubmitSummary'
import { getNetworkBadgeCopy } from '@/utils/networkDisplay'
import { LimitOrderEscrowPlaceGuardMessage } from '@/components/trade/LimitOrderEscrowPlaceGuardMessage'
import { getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { computeSwapRouteDisplay } from '@/utils/swapRouteDisplay'
import { resolveSwapRoutePairAddresses } from '@/utils/resolveSwapRoutePairAddresses'
import { TRADE_SLIPPAGE_PRESET_CLASS } from '@/utils/tradeMoneyCta'
import { TradeMarketSubmitChrome, type TradeMarketSubmitChromeModel } from '@/components/trade/TradeTicketSubmitFooter'

interface MarketSimData {
  return_amount: string
  spread_amount: string
  commission_amount: string
  quoteDisclosure: string
  indexerQuoteKind?: IndexerRouteQuoteKind
  indexerOperations?: SwapOperation[]
  routePreflight?: SwapRoutePreflightSpread
  indexerAmountReconciled?: boolean
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
}) {
  const address = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)
  const wallet = getConnectedWallet()
  const isWalletConnected = !!address && !!wallet
  const queryClient = useQueryClient()
  const { slippageTolerance, setSlippageTolerance, deadlineSeconds } = useDexStore()

  const bookLegInputId = useId()
  const maxMakersInputId = useId()

  const [marketAmountHuman, setMarketAmountHuman] = useState('')
  /** Always-on best execution (#596). Typed book leg overrides via POST. */
  const [bookInputHuman, setBookInputHuman] = useState('')
  const [hybridMaxMakers, setHybridMaxMakers] = useState(8)
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const debouncedBookInputHuman = useDebouncedValue(bookInputHuman, SIM_QUOTE_DEBOUNCE_MS)
  const debouncedHybridMaxMakers = useDebouncedValue(hybridMaxMakers, SIM_QUOTE_DEBOUNCE_MS)

  const token0 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[0]) : ''
  const token1 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[1]) : ''
  const fromToken = side === 'bid' ? token1 : token0
  const taxSell = useCommunityTaxSellBps(fromToken?.startsWith('terra1') ? fromToken : null)
  const toToken = side === 'bid' ? token0 : token1
  const taxReceive = useCommunityTaxSellBps(toToken?.startsWith('terra1') ? toToken : null)
  const offerDecimals = fromToken ? getDecimals(tokenAssetInfo(fromToken)) : 6
  const receiveDecimals = toToken ? getDecimals(tokenAssetInfo(toToken)) : 6
  const rawInputAmount = marketAmountHuman.trim() ? toRawAmount(marketAmountHuman.trim(), offerDecimals) : '0'
  const debouncedMarketAmount = useDebouncedValue(marketAmountHuman, SIM_QUOTE_DEBOUNCE_MS)
  const debouncedRawInputAmount = debouncedMarketAmount.trim()
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
      }),
    [fromToken, bookInputHuman, rawInputAmount, hybridMaxMakers]
  )
  const liveHybrid = hybridParamsFromBookSplit(liveSplit, hybridMaxMakers)

  const debouncedSplit = useMemo(
    () =>
      getDirectHybridBookSplit({
        isDirect: true,
        fromToken,
        bookInputHuman: debouncedBookInputHuman,
        rawInputAmount: debouncedRawInputAmount,
        hybridMaxMakers: debouncedHybridMaxMakers,
      }),
    [fromToken, debouncedBookInputHuman, debouncedRawInputAmount, debouncedHybridMaxMakers]
  )
  const debouncedHybrid = hybridParamsFromBookSplit(debouncedSplit, debouncedHybridMaxMakers)
  const debouncedWillSubmitHybrid = !!debouncedSplit?.willSubmitHybrid

  const bookLegMaxResult = useMemo(() => {
    if (!escrowBalanceQuery.data || rawInputAmount === '0') {
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
      evaluateLimitOrderEscrowPlaceGate(marketAmountHuman, offerDecimals, {
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

  const maxSpreadStr = useMemo(() => (slippageTolerance / 100).toString(), [slippageTolerance])
  const quoteTrader = useMemo(() => (address ? { trader: address } : undefined), [address])

  const simQuery = useQuery({
    queryKey: [
      'tradeMarketSim',
      pairAddr,
      side,
      debouncedRawInputAmount,
      debouncedBookInputHuman,
      debouncedHybridMaxMakers,
      slippageTolerance,
      address,
    ],
    placeholderData: keepPreviousData,
    queryFn: async ({ signal }): Promise<MarketSimData> => {
      if (!selectedPair || debouncedRawInputAmount === '0') throw new Error('missing')
      const simRaw = debouncedRawInputAmount
      const offerInfo = tokenAssetInfo(fromToken)
      const askInfo = tokenAssetInfo(toToken)

      // Advanced: manual book leg overrides indexer GET (same semantics as Swap).
      if (debouncedHybrid && debouncedWillSubmitHybrid) {
        if (debouncedSplit?.bookExceedsPay) throw new Error('Book leg cannot exceed pay amount')
        const quoted = await quoteDirectHybridSwap({
          pairAddress: selectedPair.contract_addr,
          fromToken,
          toToken,
          offerAssetInfo: offerInfo,
          askAssetInfo: askInfo,
          simRaw,
          hybrid: debouncedHybrid,
          maxSpreadStr,
          quoteTrader,
        })
        return {
          return_amount: quoted.return_amount,
          spread_amount: quoted.spread_amount,
          commission_amount: quoted.commission_amount,
          quoteDisclosure: quoteDisclosureForIndexerKind(quoted.indexerQuoteKind),
          indexerQuoteKind: quoted.indexerQuoteKind,
          indexerOperations: quoted.indexerOperations,
          routePreflight: quoted.routePreflight,
          indexerAmountReconciled: quoted.indexerAmountReconciled,
        }
      }

      // Default (empty manual book): GET /route/solve best-execution split (#501 / #596).
      if (fromToken.startsWith('terra1') && toToken.startsWith('terra1')) {
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
              spread_amount: quoted.spread_amount,
              commission_amount: quoted.commission_amount,
              quoteDisclosure: quoteDisclosureForIndexerKind(quoted.indexerQuoteKind),
              indexerQuoteKind: quoted.indexerQuoteKind,
              indexerOperations: quoted.indexerOperations,
              routePreflight: quoted.routePreflight,
            }
          }
        } catch {
          /* pool-only fallback below */
        }
      }

      const sim = await simulateSwap(selectedPair.contract_addr, offerInfo, simRaw, quoteTrader)
      return {
        ...sim,
        quoteDisclosure: POOL_ONLY_QUOTE_DISCLOSURE,
      }
    },
    enabled:
      !!selectedPair &&
      pairAddr.startsWith('terra1') &&
      fromToken.startsWith('terra1') &&
      toToken.startsWith('terra1') &&
      debouncedRawInputAmount !== '0',
    // Skip interval while fetching so in-flight quotes are not cancel/restarted (#484).
    refetchInterval: simQuoteRefetchInterval,
  })

  const solverHybridForGas = useMemo(
    () => hybridFromSingleHopIndexerOps(simQuery.data?.indexerOperations),
    [simQuery.data?.indexerOperations]
  )

  // Hybrid (GET or Advanced override) reserves hybrid gas; prefer settled solver / manual split params.
  const marketGasMin = useMemo(
    () => estimateMarketPairSwapSequenceUlunaFeesTotal(true, liveHybrid ?? solverHybridForGas ?? undefined),
    [liveHybrid, solverHybridForGas]
  )

  const placeNativeGasGate = useMemo(
    () =>
      evaluateMarketSwapNativeGasPlaceGate(
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

  const combinedOk = placeEscrowGate.canPlaceLimit && placeNativeGasGate.canPlaceLimit
  const inlineGate = placeEscrowGate.userMessage ? placeEscrowGate : placeNativeGasGate

  const priceImpactTooHigh = simQuery.data?.routePreflight?.anyHopExceedsMaxSpread === true

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
    simQuery,
    slippageTolerance,
    extraSubmitBlocked: priceImpactTooHigh || !!liveSplit?.bookExceedsPay,
    hybrid: {
      enabled: true,
      live: { bookInputHuman, hybridMaxMakers },
      snapshotted: hybridSubmitSnapshot,
    },
  })
  const tradeUsesRouter = communityTaxExecuteUsesRouter(simData?.indexerOperations?.length)
  const communityTaxHint = communityTaxRouteHint({
    payIsTax: taxSell.isTaxToken,
    receiveIsTax: taxReceive.isTaxToken,
    usesRouter: tradeUsesRouter,
    sellBps: taxSell.sellBps,
  })

  const swapMutation = useTerraBroadcastMutation({
    toastSuccess: 'Market swap submitted.',
    mutationFn: async () => {
      if (!address || !selectedPair) throw new Error('Connect wallet')
      if (!fromToken.startsWith('terra1')) throw new Error('Market swap requires CW20 pay token')
      assertSubmitQuotePayRawAligned(rawInputAmount, debouncedRawInputAmount)
      if (snapshottedHybrid) {
        assertSubmitHybridAligned({ bookInputHuman, hybridMaxMakers }, snapshottedHybrid)
      }
      if (!simData) throw new Error('Quote unavailable')
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
      const deadline = Math.floor(Date.now() / 1000) + deadlineSeconds
      const idxOps = simData.indexerOperations
      return executeCw20AllowanceThen(address, fromToken, selectedPair.contract_addr, payRaw, async () => {
        if (swapOpsRequireRouter(idxOps)) {
          const opsForSubmit = await enrichSwapOperationsWithHopMinReturns(
            idxOps!,
            payRaw,
            slippageTolerance,
            address ? { trader: address } : undefined
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
        // Prefer solver hybrid from GET quote; Advanced manual split is the fallback (#501).
        // Cap max_maker_fills like Swap so gas preflight matches the on-chain payload (#249).
        let hopHybrid = hybridFromSingleHopIndexerOps(idxOps) ?? debouncedHybrid ?? undefined
        if (hopHybrid) {
          hopHybrid = hybridParamsWithSubmitCap(hopHybrid)
        }
        const directMinReturn =
          hopHybrid && BigInt(hopHybrid.book_input) > 0n
            ? await computeDirectHybridMinReturn(
                selectedPair.contract_addr,
                tokenAssetInfo(fromToken),
                payRaw,
                hopHybrid,
                slippageTolerance,
                quoteTrader
              )
            : undefined
        return swap(address, fromToken, selectedPair.contract_addr, payRaw, undefined, maxSpread, undefined, {
          hybrid: hopHybrid,
          minReturn: directMinReturn,
          deadline,
        })
      })
    },
    onSuccess: () => {
      sounds.playSuccess()
      setMarketAmountHuman('')
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

  const receiveHuman =
    simData?.return_amount != null && simData.return_amount !== ''
      ? formatTokenAmount(simData.return_amount, receiveDecimals, 6)
      : '—'

  const minReceiveHuman =
    minReceived != null && minReceived !== '' ? formatTokenAmount(minReceived, receiveDecimals, 6) : '—'

  // Same receive stale/loading rules as Swap (#484 keep-previous vs #496 pay change).
  const hasSettledSimQuote = !!simQuery.data && !simQuery.isPlaceholderData
  const showReceiveCalculating = shouldShowSimReceiveCalculating(
    simQuery.isFetching,
    hasSettledSimQuote,
    simQuery.isPlaceholderData,
    rawInputAmount !== debouncedRawInputAmount
  )

  const canSubmit =
    isWalletConnected &&
    !isPaused &&
    combinedOk &&
    !swapMutation.isPending &&
    !!selectedPair &&
    rawInputAmount !== '0' &&
    isSubmitReady

  const submitLabel = !isWalletConnected
    ? 'Connect Wallet'
    : terraBroadcastPendingButtonLabel(
        swapMutation.phase,
        swapMutation.isPending,
        priceImpactTooHigh
          ? 'Hop spread exceeds slippage protection'
          : liveSplit?.bookExceedsPay
            ? 'Book leg exceeds pay'
            : `Market ${side === 'bid' ? 'buy' : 'sell'}`,
        'Submitting…'
      )

  const swapMutate = swapMutation.mutate
  const swapPhase = swapMutation.phase
  const swapPendingTxHash = swapMutation.pendingTxHash
  const swapIsError = swapMutation.isError
  const swapErrorMessage = swapMutation.isError ? (swapMutation.error as Error).message : null
  const swapIsSuccess = swapMutation.isSuccess
  const swapSuccessTxHash = swapMutation.data

  const submitChromeModel = useMemo<TradeMarketSubmitChromeModel>(
    () => ({
      canSubmit,
      label: submitLabel,
      onClick: () => {
        if (!interactive) return
        if (!isWalletConnected) openWalletModal()
        else swapMutate()
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
    <div className="space-y-3 border-t border-white/10 pt-3">
      <h3 className="text-xs font-semibold uppercase tracking-wide">Market</h3>
      <p className="text-[10px] leading-snug" style={{ color: 'var(--ink-dim)' }}>
        Taker swap at {slippageTolerance}% {SLIPPAGE_PROTECTION_LABEL.toLowerCase()}. Best pool/book split by default.{' '}
        <a
          className="underline hover:opacity-80"
          href={`${DOCS_GITLAB_BASE}/limit-orders.md`}
          target="_blank"
          rel="noopener noreferrer"
        >
          Docs
        </a>
      </p>
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
        extraDebitSellBps={extraDebitSellBpsForExecute(taxSell.sellBps, tradeUsesRouter)}
      />
      {communityTaxHint && (
        <p className="text-[10px]" style={{ color: 'var(--ink-dim)' }} data-testid="trade-sell-tax-extra">
          {communityTaxHint}
        </p>
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
                {isWalletConnected && fromToken.startsWith('terra1') && (
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
          </div>
        )}
      </div>

      {(simQuery.isLoading || showReceiveCalculating) && rawInputAmount !== '0' && (
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
      {simQuery.isError && rawInputAmount !== '0' && !showReceiveCalculating && (
        <p className="text-[10px] alert-error" role="alert" data-testid="trade-market-quote-error">
          {humanizeUserFacingErrorFromUnknown(simQuery.error)}
        </p>
      )}
      {liveSplit?.bookExceedsPay && (
        <p className="text-[10px] alert-error" role="alert" data-testid="trade-market-book-exceeds-pay">
          Book leg cannot exceed pay amount
        </p>
      )}
      {simQuery.data && rawInputAmount !== '0' && (
        <div className="card-glass !p-2 space-y-1 text-[10px]" data-testid="trade-market-quote">
          <div className="flex justify-between gap-2">
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
              <p style={{ color: indexerHybridExec.degraded ? 'var(--color-warning, #f59e0b)' : 'var(--ink-dim)' }}>
                {indexerHybridExec.line}
              </p>
            </div>
          )}
          {simQuery.data.routePreflight && (
            <p style={{ color: 'var(--ink-dim)' }}>
              Worst hop spread (sim): {simQuery.data.routePreflight.worstSpreadPercent}%
            </p>
          )}
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
          offerAmountHuman={marketAmountHuman}
          receiveAmountHuman={receiveHuman}
          maxSpreadPercent={slippageTolerance}
          minReceiveHuman={minReceived != null && minReceived !== '' ? minReceiveHuman : null}
          pairContractAddresses={marketPairContractAddresses}
          chainFullLabel={getNetworkBadgeCopy().fullLabel}
          data-testid="trade-market-pre-submit-summary"
        />
      )}

      <LimitOrderEscrowPlaceGuardMessage gate={inlineGate} data-testid="trade-market-place-guard" />
      {!dockSubmit && <TradeMarketSubmitChrome model={submitChromeModel} />}
    </div>
  )
}
