import { useMemo, useState, useId } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { useDexStore } from '@/stores/dex'
import { useLimitOrderEscrowBalance } from '@/hooks/useLimitOrderEscrowBalance'
import { useNativeUlunaBalance } from '@/hooks/useNativeUlunaBalance'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { simulateSwap, simulateHybridSwap, swap } from '@/services/terraclassic/pair'
import { preflightSwapRouteSpread, type SwapRoutePreflightSpread } from '@/services/terraclassic/swapRoutePreflight'
import { executeMultiHopSwap, type SwapOperation } from '@/services/terraclassic/router'
import { hybridParamsWithSubmitCap } from '@/services/terraclassic/hybridSwapGas'
import { hybridFromSingleHopIndexerOps, swapOpsRequireRouter } from '@/services/terraclassic/swapRouting'
import {
  executeCw20AllowanceThen,
  estimateMarketPairSwapSequenceUlunaFeesTotal,
} from '@/services/terraclassic/transactions'
import { postRouteSolve } from '@/services/indexer/client'
import { swapOperationsFromIndexerResponse } from '@/services/indexer/routeOperations'
import { sounds } from '@/lib/sounds'
import { TxResultAlert, Spinner } from '@/components/ui'
import {
  assetInfoLabel,
  tokenAssetInfo,
  type HybridSwapParams,
  type IndexerRouteQuoteKind,
  type PairInfo,
} from '@/types'
import { getDecimals, toRawAmount, fromRawAmount, formatTokenAmount } from '@/utils/formatAmount'
import { applySlippagePercentFloor } from '@/utils/rawAmountMath'
import { isDecimalAmountDraft, tryParseBigInt } from '@/utils/decimalAmountInput'
import { computeMaxSpendableHumanAmount } from '@/utils/maxSpendableAmount'
import { AmountBalanceActions } from '@/components/common/AmountBalanceActions'
import { evaluateLimitOrderEscrowPlaceGate } from '@/utils/limitOrderEscrowBalanceGate'
import { evaluateMarketSwapNativeGasPlaceGate } from '@/utils/limitOrderNativeGasBalanceGate'
import { getDirectHybridBookSplit, getIndexerHybridExecutionSummary } from '@/utils/swapDisclosure'
import { LimitOrderEscrowAmountField } from '@/components/trade/LimitOrderEscrowAmountField'
import { LimitOrderEscrowPlaceGuardMessage } from '@/components/trade/LimitOrderEscrowPlaceGuardMessage'
import { getTokenDisplaySymbol } from '@/utils/tokenDisplay'

interface MarketSimData {
  return_amount: string
  spread_amount: string
  commission_amount: string
  quoteDisclosure: string
  indexerQuoteKind?: IndexerRouteQuoteKind
  indexerOperations?: SwapOperation[]
  receiveQuoteIsPoolOnlyWithConfiguredBookLeg?: boolean
  routePreflight?: SwapRoutePreflightSpread
}

function quoteDisclosureForIndexerKind(kind: IndexerRouteQuoteKind | undefined): string {
  switch (kind) {
    case 'indexer_hybrid_lcd_degraded':
      return 'Indexer hybrid route (LCD) — one or more hops fell back to pool-only on the indexer.'
    case 'indexer_hybrid_lcd':
      return 'Indexer-optimized hybrid splits · quoted via your wallet LCD simulation (matches submit shape).'
    case 'indexer_pool_lcd':
      return 'Indexer route (pool-only legs) · quoted via your wallet LCD simulation.'
    case 'indexer_route_only':
      return 'Indexer-solved route · no aggregate router estimate (simulation unavailable).'
    default:
      return 'Quoted via your wallet (chain simulation).'
  }
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
  side,
  isPaused,
}: {
  pairAddr: string
  selectedPair: PairInfo | undefined
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

  const token0 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[0]) : ''
  const token1 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[1]) : ''
  const fromToken = side === 'bid' ? token1 : token0
  const toToken = side === 'bid' ? token0 : token1
  const offerDecimals = fromToken ? getDecimals(tokenAssetInfo(fromToken)) : 6
  const receiveDecimals = toToken ? getDecimals(tokenAssetInfo(toToken)) : 6
  const rawInputAmount = marketAmountHuman.trim() ? toRawAmount(marketAmountHuman.trim(), offerDecimals) : '0'

  const escrowBalanceQuery = useLimitOrderEscrowBalance(address, fromToken)
  const nativeUlunaQuery = useNativeUlunaBalance(address)

  const { hybrid, willSubmitHybrid } = useMemo(
    () => computeHybridParams(rawInputAmount, fromToken, useHybridBook, bookInputHuman, hybridMaxMakers),
    [rawInputAmount, fromToken, useHybridBook, bookInputHuman, hybridMaxMakers]
  )

  const marketGasMin = useMemo(
    () => estimateMarketPairSwapSequenceUlunaFeesTotal(willSubmitHybrid, hybrid),
    [willSubmitHybrid, hybrid]
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
      rawInputAmount,
      useHybridBook,
      bookInputHuman,
      hybridMaxMakers,
      slippageTolerance,
      address,
    ],
    queryFn: async (): Promise<MarketSimData> => {
      if (!selectedPair || rawInputAmount === '0') throw new Error('missing')
      const offerInfo = tokenAssetInfo(fromToken)
      const askInfo = tokenAssetInfo(toToken)

      if (useHybridBook && hybrid && willSubmitHybrid) {
        try {
          const idx = await postRouteSolve(
            fromToken,
            toToken,
            rawInputAmount,
            [
              {
                pool_input: hybrid.pool_input,
                book_input: hybrid.book_input,
                max_maker_fills: hybrid.max_maker_fills,
                book_start_hint: hybrid.book_start_hint,
              },
            ],
            quoteTrader
          )
          if (idx.estimated_amount_out?.trim()) {
            const ops = swapOperationsFromIndexerResponse(idx.router_operations as unknown[], idx.hops.length)
            const routePreflight =
              ops.length > 0
                ? await preflightSwapRouteSpread(ops, rawInputAmount, maxSpreadStr, quoteTrader)
                : undefined
            return {
              return_amount: idx.estimated_amount_out,
              spread_amount: '0',
              commission_amount: '0',
              quoteDisclosure: quoteDisclosureForIndexerKind(idx.quote_kind),
              indexerQuoteKind: idx.quote_kind,
              indexerOperations: ops.length > 0 ? ops : undefined,
              receiveQuoteIsPoolOnlyWithConfiguredBookLeg: false,
              routePreflight,
            }
          }
        } catch {
          /* fall through */
        }
        try {
          const sim = await simulateHybridSwap(
            selectedPair.contract_addr,
            offerInfo,
            rawInputAmount,
            hybrid,
            quoteTrader
          )
          const ops: SwapOperation[] = [
            {
              terra_swap: {
                offer_asset_info: offerInfo,
                ask_asset_info: askInfo,
                hybrid,
              },
            },
          ]
          const routePreflight = await preflightSwapRouteSpread(ops, rawInputAmount, maxSpreadStr, quoteTrader)
          return {
            return_amount: sim.return_amount,
            spread_amount: sim.spread_amount,
            commission_amount: sim.commission_amount,
            quoteDisclosure: 'Direct pair · hybrid_simulation (Pattern C).',
            routePreflight,
            receiveQuoteIsPoolOnlyWithConfiguredBookLeg: false,
          }
        } catch {
          /* pool-only fallback */
        }
      }

      const sim = await simulateSwap(selectedPair.contract_addr, offerInfo, rawInputAmount, quoteTrader)
      const hybridSplit = getDirectHybridBookSplit({
        isDirect: true,
        useHybridBook,
        fromToken,
        bookInputHuman: bookInputHuman.trim() ? bookInputHuman : fromRawAmount(rawInputAmount, offerDecimals),
        rawInputAmount,
        hybridMaxMakers,
      })
      return {
        ...sim,
        quoteDisclosure: 'Direct pair · hybrid_simulation (pool-only leg; book not in quote).',
        receiveQuoteIsPoolOnlyWithConfiguredBookLeg: !!(hybridSplit?.willSubmitHybrid && !hybridSplit?.bookExceedsPay),
      }
    },
    enabled:
      !!selectedPair &&
      pairAddr.startsWith('terra1') &&
      fromToken.startsWith('terra1') &&
      toToken.startsWith('terra1') &&
      rawInputAmount !== '0',
    refetchInterval: 10_000,
  })

  const minReceived = useMemo(() => {
    if (!simQuery.data?.return_amount) return null
    return applySlippagePercentFloor(simQuery.data.return_amount, slippageTolerance)
  }, [simQuery.data?.return_amount, slippageTolerance])

  const swapMutation = useMutation({
    mutationFn: async () => {
      if (!address || !selectedPair) throw new Error('Connect wallet')
      if (!fromToken.startsWith('terra1')) throw new Error('Market swap requires CW20 pay token')
      const escrowGate = evaluateLimitOrderEscrowPlaceGate(marketAmountHuman, offerDecimals, escrowBalanceQuery)
      if (!escrowGate.canPlaceLimit) {
        if (!escrowGate.userMessage) throw new Error('Enter amount')
        throw new Error(escrowGate.userMessage)
      }
      const raw = toRawAmount(marketAmountHuman.trim(), offerDecimals)
      if (raw === '0') throw new Error('Enter amount')
      const nativeGate = evaluateMarketSwapNativeGasPlaceGate(
        marketAmountHuman,
        offerDecimals,
        nativeUlunaQuery,
        marketGasMin,
        willSubmitHybrid ? 'hybrid swap' : 'swap'
      )
      if (!nativeGate.canPlaceLimit) {
        if (!nativeGate.userMessage) throw new Error('Insufficient LUNC for gas')
        throw new Error(nativeGate.userMessage)
      }
      const maxSpread = maxSpreadStr
      const deadline = Math.floor(Date.now() / 1000) + deadlineSeconds
      const idxOps = simQuery.data?.indexerOperations
      const submitHybrid = hybrid ? hybridParamsWithSubmitCap(hybrid) : undefined
      return executeCw20AllowanceThen(address, fromToken, selectedPair.contract_addr, raw, async () => {
        if (swapOpsRequireRouter(idxOps)) {
          return executeMultiHopSwap(
            address,
            fromToken,
            raw,
            idxOps!,
            maxSpread,
            minReceived ?? undefined,
            undefined,
            deadline
          )
        }
        const hopHybrid = hybridFromSingleHopIndexerOps(idxOps) ?? submitHybrid
        return swap(address, fromToken, selectedPair.contract_addr, raw, undefined, maxSpread, undefined, {
          hybrid: hopHybrid,
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

  const receiveHuman =
    simQuery.data?.return_amount != null && simQuery.data.return_amount !== ''
      ? formatTokenAmount(simQuery.data.return_amount, receiveDecimals, 6)
      : '—'

  const minReceiveHuman =
    minReceived != null && minReceived !== '' ? formatTokenAmount(minReceived, receiveDecimals, 6) : '—'

  const priceImpactTooHigh = simQuery.data?.routePreflight?.anyHopExceedsMaxSpread === true

  const canSubmit =
    isWalletConnected &&
    !isPaused &&
    combinedOk &&
    !swapMutation.isPending &&
    !!selectedPair &&
    rawInputAmount !== '0' &&
    !simQuery.isLoading &&
    !simQuery.isError &&
    !!simQuery.data &&
    !priceImpactTooHigh

  if (!selectedPair) return null

  return (
    <div className="space-y-3 border-t border-white/10 pt-3">
      <h3 className="text-xs font-semibold uppercase tracking-wide">Market</h3>
      <p className="text-[10px] leading-snug" style={{ color: 'var(--ink-dim)' }}>
        Executes a taker swap on this pair using your global slippage cap (
        <span className="font-mono">{slippageTolerance}%</span>
        ). Hybrid routing walks the on-chain limit book first, then the pool (Pattern C — see docs/limit-orders.md).
      </p>
      <div className="flex flex-wrap gap-2 text-[10px]">
        <span style={{ color: 'var(--ink-dim)' }}>Slippage:</span>
        {[0.1, 0.5, 1.0].map((v) => (
          <button
            key={v}
            type="button"
            className={`tab-neo !text-[10px] !px-2 !py-0.5 ${slippageTolerance === v ? 'tab-neo-active' : 'tab-neo-inactive'}`}
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
          <p className="text-[10px]" style={{ color: 'var(--ink-dim)' }}>
            Book leg defaults to the full pay amount when the field below is empty (book first, pool takes the
            remainder).
          </p>
          <div>
            <label className="label-neo text-[10px]" htmlFor={bookLegInputId}>
              Book leg override ({getTokenDisplaySymbol(fromToken)})
            </label>
            <input
              id={bookLegInputId}
              type="text"
              inputMode="decimal"
              className="input-neo !text-xs w-full font-mono"
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
            <label className="label-neo text-[10px]" htmlFor={maxMakersInputId}>
              Max distinct makers
            </label>
            <input
              id={maxMakersInputId}
              type="number"
              className="input-neo !text-xs w-full"
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
        <p className="text-[10px] alert-error">{(simQuery.error as Error).message}</p>
      )}
      {simQuery.data && rawInputAmount !== '0' && (
        <div className="card-neo !p-2 space-y-1 text-[10px]" data-testid="trade-market-quote">
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
          {simQuery.data.receiveQuoteIsPoolOnlyWithConfiguredBookLeg && (
            <p
              className="text-[10px] font-semibold uppercase tracking-wide"
              style={{ color: 'var(--color-warning, #f59e0b)' }}
            >
              Quote line is pool-only while a book leg is configured — submitted hybrid fill may differ (L8).
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
        </div>
      )}

      <button
        type="button"
        className="btn-primary btn-cta w-full !text-xs"
        disabled={!canSubmit}
        data-testid="trade-market-submit"
        onClick={() => {
          if (!isWalletConnected) openWalletModal()
          else swapMutation.mutate()
        }}
      >
        {!isWalletConnected
          ? 'Connect Wallet'
          : swapMutation.isPending
            ? 'Submitting…'
            : priceImpactTooHigh
              ? 'Price impact too high'
              : `Market ${side === 'bid' ? 'buy' : 'sell'}`}
      </button>
      <LimitOrderEscrowPlaceGuardMessage gate={inlineGate} data-testid="trade-market-place-guard" />
      {swapMutation.isError && <TxResultAlert type="error" message={(swapMutation.error as Error).message} />}
      {swapMutation.isSuccess && (
        <TxResultAlert type="success" message="Market swap submitted." txHash={swapMutation.data} />
      )}
    </div>
  )
}
