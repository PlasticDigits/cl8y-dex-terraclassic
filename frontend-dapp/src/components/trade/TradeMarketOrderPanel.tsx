import { useMemo, useState, useId } from 'react'
import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { useDebouncedValue } from '@/hooks/useDebouncedValue'
import { assertSubmitHybridAligned, assertSubmitQuotePayRawAligned, SIM_QUOTE_DEBOUNCE_MS } from '@/utils/quoteDebounce'
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
} from '@/utils/directHybridQuote'
import { humanizeUserFacingErrorFromUnknown } from '@/utils/humanizeUserFacingError'
import { DOCS_GITLAB_BASE } from '@/utils/constants'
import { sounds } from '@/lib/sounds'
import { SLIPPAGE_PROTECTION_LABEL } from '@/utils/slippageProtectionCopy'
import { TxResultAlert, Spinner } from '@/components/ui'
import { TerraBroadcastPendingLink } from '@/components/ui/TerraBroadcastPendingLink'
import { terraBroadcastPendingButtonLabel } from '@/utils/terraBroadcastUi'
import {
  assetInfoLabel,
  tokenAssetInfo,
  type HybridSwapParams,
  type IndexerRouteQuoteKind,
  type PairInfo,
} from '@/types'
import { getDecimals, toRawAmount, formatTokenAmount } from '@/utils/formatAmount'
import { isDecimalAmountDraft, tryParseBigInt } from '@/utils/decimalAmountInput'
import { computeMaxSpendableHumanAmount } from '@/utils/maxSpendableAmount'
import { AmountBalanceActions } from '@/components/common/AmountBalanceActions'
import { evaluateLimitOrderEscrowPlaceGate } from '@/utils/limitOrderEscrowBalanceGate'
import { evaluateMarketSwapNativeGasPlaceGate } from '@/utils/limitOrderNativeGasBalanceGate'
import { getIndexerHybridExecutionSummary } from '@/utils/swapDisclosure'
import { LimitOrderEscrowAmountField } from '@/components/trade/LimitOrderEscrowAmountField'
import { SwapPreSubmitSummary } from '@/components/swap/SwapPreSubmitSummary'
import { getNetworkBadgeCopy } from '@/utils/networkDisplay'
import { LimitOrderEscrowPlaceGuardMessage } from '@/components/trade/LimitOrderEscrowPlaceGuardMessage'
import { getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { computeSwapRouteDisplay } from '@/utils/swapRouteDisplay'
import { resolveSwapRoutePairAddresses } from '@/utils/resolveSwapRoutePairAddresses'
import { TRADE_MONEY_CTA_CLASS, TRADE_SLIPPAGE_PRESET_CLASS } from '@/utils/tradeMoneyCta'

interface MarketSimData {
  return_amount: string
  spread_amount: string
  commission_amount: string
  quoteDisclosure: string
  indexerQuoteKind?: IndexerRouteQuoteKind
  indexerOperations?: SwapOperation[]
  routePreflight?: SwapRoutePreflightSpread
}

function computeHybridParams(
  rawTotal: string,
  fromToken: string,
  useHybridBook: boolean,
  bookInputHuman: string,
  hybridMaxMakers: number
): { hybrid: HybridSwapParams | undefined; willSubmitHybrid: boolean } {
  if (!useHybridBook || !fromToken.startsWith('terra1')) {
    return { hybrid: undefined, willSubmitHybrid: false }
  }
  const dec = getDecimals(tokenAssetInfo(fromToken))
  const bookHuman = bookInputHuman.trim()
  if (bookHuman && !isDecimalAmountDraft(bookHuman)) {
    return { hybrid: undefined, willSubmitHybrid: false }
  }
  const bookRawStr = bookHuman ? toRawAmount(bookHuman, dec) : rawTotal
  const total = tryParseBigInt(rawTotal)
  const book = tryParseBigInt(bookRawStr)
  if (total === null || book === null) {
    return { hybrid: undefined, willSubmitHybrid: false }
  }
  if (book <= 0n) return { hybrid: undefined, willSubmitHybrid: false }
  if (book > total) return { hybrid: undefined, willSubmitHybrid: false }
  const pool = total - book
  if (hybridMaxMakers < 1) return { hybrid: undefined, willSubmitHybrid: false }
  return {
    hybrid: {
      pool_input: pool.toString(),
      book_input: book.toString(),
      max_maker_fills: hybridMaxMakers,
      book_start_hint: null,
    },
    willSubmitHybrid: true,
  }
}

export function TradeMarketOrderPanel({
  pairAddr,
  selectedPair,
  pairs,
  side,
  isPaused,
}: {
  pairAddr: string
  selectedPair: PairInfo | undefined
  pairs: PairInfo[]
  side: 'bid' | 'ask'
  isPaused: boolean
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
  const [useHybridBook, setUseHybridBook] = useState(true)
  const [bookInputHuman, setBookInputHuman] = useState('')
  const [hybridMaxMakers, setHybridMaxMakers] = useState(8)
  const debouncedBookInputHuman = useDebouncedValue(bookInputHuman, SIM_QUOTE_DEBOUNCE_MS)
  const debouncedHybridMaxMakers = useDebouncedValue(hybridMaxMakers, SIM_QUOTE_DEBOUNCE_MS)

  const token0 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[0]) : ''
  const token1 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[1]) : ''
  const fromToken = side === 'bid' ? token1 : token0
  const toToken = side === 'bid' ? token0 : token1
  const offerDecimals = fromToken ? getDecimals(tokenAssetInfo(fromToken)) : 6
  const receiveDecimals = toToken ? getDecimals(tokenAssetInfo(toToken)) : 6
  const rawInputAmount = marketAmountHuman.trim() ? toRawAmount(marketAmountHuman.trim(), offerDecimals) : '0'
  const debouncedMarketAmount = useDebouncedValue(marketAmountHuman, SIM_QUOTE_DEBOUNCE_MS)
  const debouncedRawInputAmount = debouncedMarketAmount.trim()
    ? toRawAmount(debouncedMarketAmount.trim(), offerDecimals)
    : '0'

  const escrowBalanceQuery = useLimitOrderEscrowBalance(address, fromToken)
  const nativeUlunaQuery = useNativeUlunaBalance(address)

  const { hybrid: liveHybrid, willSubmitHybrid } = useMemo(
    () => computeHybridParams(rawInputAmount, fromToken, useHybridBook, bookInputHuman, hybridMaxMakers),
    [rawInputAmount, fromToken, useHybridBook, bookInputHuman, hybridMaxMakers]
  )

  const { hybrid: debouncedHybrid, willSubmitHybrid: debouncedWillSubmitHybrid } = useMemo(
    () =>
      computeHybridParams(
        debouncedRawInputAmount,
        fromToken,
        useHybridBook,
        debouncedBookInputHuman,
        debouncedHybridMaxMakers
      ),
    [debouncedRawInputAmount, fromToken, useHybridBook, debouncedBookInputHuman, debouncedHybridMaxMakers]
  )

  const marketGasMin = useMemo(
    () => estimateMarketPairSwapSequenceUlunaFeesTotal(willSubmitHybrid, liveHybrid),
    [willSubmitHybrid, liveHybrid]
  )

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
        willSubmitHybrid ? 'hybrid swap' : 'swap'
      ),
    [
      marketAmountHuman,
      offerDecimals,
      nativeUlunaQuery.data,
      nativeUlunaQuery.isLoading,
      nativeUlunaQuery.isError,
      marketGasMin,
      willSubmitHybrid,
    ]
  )

  const combinedOk = placeEscrowGate.canPlaceLimit && placeNativeGasGate.canPlaceLimit
  const inlineGate = placeEscrowGate.userMessage ? placeEscrowGate : placeNativeGasGate

  const maxSpreadStr = useMemo(() => (slippageTolerance / 100).toString(), [slippageTolerance])
  const quoteTrader = useMemo(() => (address ? { trader: address } : undefined), [address])

  const simQuery = useQuery({
    queryKey: [
      'tradeMarketSim',
      pairAddr,
      side,
      debouncedRawInputAmount,
      useHybridBook,
      debouncedBookInputHuman,
      debouncedHybridMaxMakers,
      slippageTolerance,
      address,
    ],
    placeholderData: keepPreviousData,
    queryFn: async (): Promise<MarketSimData> => {
      if (!selectedPair || debouncedRawInputAmount === '0') throw new Error('missing')
      const simRaw = debouncedRawInputAmount
      const offerInfo = tokenAssetInfo(fromToken)
      const askInfo = tokenAssetInfo(toToken)

      if (useHybridBook && debouncedHybrid && debouncedWillSubmitHybrid) {
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
    refetchInterval: 10_000,
  })

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
    extraSubmitBlocked: priceImpactTooHigh,
    hybrid: useHybridBook
      ? {
          enabled: true,
          live: { bookInputHuman, hybridMaxMakers },
          snapshotted: hybridSubmitSnapshot,
        }
      : undefined,
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
        debouncedWillSubmitHybrid ? 'hybrid swap' : 'swap'
      )
      if (!nativeGate.canPlaceLimit) {
        if (!nativeGate.userMessage) throw new Error('Insufficient LUNC for gas')
        throw new Error(nativeGate.userMessage)
      }
      const maxSpread = maxSpreadStr
      const deadline = Math.floor(Date.now() / 1000) + deadlineSeconds
      const idxOps = simData.indexerOperations
      const submitHybrid = debouncedHybrid ? hybridParamsWithSubmitCap(debouncedHybrid) : undefined
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
        const hopHybrid = hybridFromSingleHopIndexerOps(idxOps) ?? submitHybrid
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

  const canSubmit =
    isWalletConnected &&
    !isPaused &&
    combinedOk &&
    !swapMutation.isPending &&
    !!selectedPair &&
    rawInputAmount !== '0' &&
    isSubmitReady

  if (!selectedPair) return null

  return (
    <div className="space-y-3 border-t border-white/10 pt-3">
      <h3 className="text-xs font-semibold uppercase tracking-wide">Market</h3>
      <p className="text-[10px] leading-snug" style={{ color: 'var(--ink-dim)' }}>
        Executes a taker swap on this pair using your global {SLIPPAGE_PROTECTION_LABEL.toLowerCase()} (
        <span className="font-mono">{slippageTolerance}%</span>
        ). When hybrid routing is enabled, your trade fills against the on-chain limit book first, then the pool.{' '}
        <a
          className="underline hover:opacity-80"
          href={`${DOCS_GITLAB_BASE}/limit-orders.md`}
          target="_blank"
          rel="noopener noreferrer"
        >
          Learn more
        </a>
      </p>
      <div className="flex flex-wrap gap-2 text-[10px]">
        <span style={{ color: 'var(--ink-dim)' }}>{SLIPPAGE_PROTECTION_LABEL}:</span>
        {[0.1, 0.5, 1.0].map((v) => (
          <button
            key={v}
            type="button"
            className={`${TRADE_SLIPPAGE_PRESET_CLASS} ${slippageTolerance === v ? 'tab-glass-active' : 'tab-glass-inactive'}`}
            data-testid={`trade-market-slippage-preset-${v}`}
            onClick={() => {
              sounds.playButtonPress()
              setSlippageTolerance(v)
            }}
          >
            {v}%
          </button>
        ))}
      </div>
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
        marketUsesHybrid={willSubmitHybrid}
      />
      <label className="flex items-center gap-2 text-[11px] cursor-pointer">
        <input type="checkbox" checked={useHybridBook} onChange={(e) => setUseHybridBook(e.target.checked)} />
        Use hybrid book + pool routing
      </label>
      {useHybridBook && (
        <div className="space-y-2">
          <p
            className="text-[10px] leading-snug"
            style={{ color: 'var(--ink-dim)' }}
            data-testid="trade-market-hybrid-min-return-notice"
          >
            Hybrid routing walks the limit book first, then fills any remainder from the pool in the same transaction.
            Your <strong>min return</strong> (from slippage protection above) is enforced on the{' '}
            <strong>combined</strong> book + pool payout after fees — the chain rejects the swap if net output is too
            low. When both legs are used, at least <strong>10%</strong> of your pay amount must go through the pool.
          </p>
          <p className="text-[10px]" style={{ color: 'var(--ink-dim)' }}>
            Book leg defaults to the full pay amount when the field below is empty (book first, pool takes the
            remainder).
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
              placeholder="Leave empty for 100% book leg"
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
              Max distinct makers
              <span
                className="ml-1 cursor-help opacity-70"
                title="Caps how many resting limits one swap can fill. If the book cannot satisfy your size within this cap, the remainder spills to the pool leg in the same transaction."
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
            />
          </div>
        </div>
      )}

      {simQuery.isLoading && rawInputAmount !== '0' && (
        <div className="flex items-center gap-2 text-[10px]" style={{ color: 'var(--ink-dim)' }}>
          <Spinner />
          Quoting…
        </div>
      )}
      {simQuery.isError && rawInputAmount !== '0' && (
        <p className="text-[10px] alert-error" role="alert" data-testid="trade-market-quote-error">
          {humanizeUserFacingErrorFromUnknown(simQuery.error)}
        </p>
      )}
      {simQuery.data && rawInputAmount !== '0' && (
        <div className="card-glass !p-2 space-y-1 text-[10px]" data-testid="trade-market-quote">
          <div className="flex justify-between gap-2">
            <span style={{ color: 'var(--ink-dim)' }}>Expected receive</span>
            <span className="font-mono text-right">
              {receiveHuman} {getTokenDisplaySymbol(toToken)}
            </span>
          </div>
          <div className="flex justify-between gap-2">
            <span style={{ color: 'var(--ink-dim)' }}>Min. after slippage</span>
            <span className="font-mono text-right">
              {minReceiveHuman} {getTokenDisplaySymbol(toToken)}
            </span>
          </div>
          <p style={{ color: 'var(--ink-subtle)' }}>{simQuery.data.quoteDisclosure}</p>
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

      {simQuery.data && rawInputAmount !== '0' && fromToken && toToken && (
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

      <button
        type="button"
        className={TRADE_MONEY_CTA_CLASS}
        disabled={!canSubmit}
        data-testid="trade-market-submit"
        onClick={() => {
          if (!isWalletConnected) openWalletModal()
          else swapMutation.mutate()
        }}
      >
        {!isWalletConnected
          ? 'Connect Wallet'
          : terraBroadcastPendingButtonLabel(
              swapMutation.phase,
              swapMutation.isPending,
              priceImpactTooHigh
                ? 'Hop spread exceeds slippage protection'
                : `Market ${side === 'bid' ? 'buy' : 'sell'}`,
              'Submitting…'
            )}
      </button>
      <TerraBroadcastPendingLink phase={swapMutation.phase} txHash={swapMutation.pendingTxHash} />
      <LimitOrderEscrowPlaceGuardMessage gate={inlineGate} data-testid="trade-market-place-guard" />
      {swapMutation.isError && <TxResultAlert type="error" message={(swapMutation.error as Error).message} />}
      {swapMutation.isSuccess && (
        <TxResultAlert type="success" message="Market swap confirmed." txHash={swapMutation.data} />
      )}
    </div>
  )
}
