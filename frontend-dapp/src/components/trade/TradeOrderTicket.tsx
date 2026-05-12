import { useMemo, useState, useEffect, useId } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { usePairLimitCancellations } from '@/hooks/usePairLimitCancellations'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { placeLimitOrder, cancelLimitOrder, getPairPaused } from '@/services/terraclassic/pair'
import {
  executeTerraContract,
  estimateLimitOrderPlaceSequenceUlunaFeesTotal,
} from '@/services/terraclassic/transactions'
import { getPairLimitPlacements, getOraclePrice } from '@/services/indexer/client'
import { sounds } from '@/lib/sounds'
import { TxResultAlert, Spinner } from '@/components/ui'
import { assetInfoLabel, tokenAssetInfo, type IndexerLimitCancellation, type IndexerPair, type PairInfo } from '@/types'
import { getDecimals, toRawAmount } from '@/utils/formatAmount'
import { evaluateLimitOrderEscrowPlaceGate } from '@/utils/limitOrderEscrowBalanceGate'
import { evaluateLimitOrderNativeGasPlaceGate } from '@/utils/limitOrderNativeGasBalanceGate'
import { warnIndexerPlacementPollFailed } from '@/utils/warnIndexerPlacementPollFailed'
import { orderIdHasIndexedCancellation } from '@/utils/limitOrderCancelUserMessage'
import { fetchCW20TokenInfo, getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { DOCS_GITLAB_BASE } from '@/utils/constants'
import { useLimitOrderForm } from '@/hooks/useLimitOrderForm'
import { useLimitOrderEscrowBalance } from '@/hooks/useLimitOrderEscrowBalance'
import { useNativeUlunaBalance } from '@/hooks/useNativeUlunaBalance'
import { LimitOrderEscrowAmountField } from '@/components/trade/LimitOrderEscrowAmountField'
import { LimitOrderExpiryField } from '@/components/trade/LimitOrderExpiryField'
import { LimitOrderAdvancedLimitSettings } from '@/components/trade/LimitOrderAdvancedLimitSettings'
import { LimitOrderEscrowPlaceGuardMessage } from '@/components/trade/LimitOrderEscrowPlaceGuardMessage'
import { LimitOrderMyPlacementsPanel } from '@/components/trade/LimitOrderMyPlacementsPanel'
import { useTradeBestBookPrices } from '@/hooks/useTradeBestBookPrices'
import { describeLimitCrossingBlocker } from '@/utils/limitOrderNonCrossing'
import { inverseLimitPriceHuman, limitPriceUsdHint } from '@/utils/tradeLimitPriceDisplay'
import { TradeMarketOrderPanel } from '@/components/trade/TradeMarketOrderPanel'

/**
 * Trade workspace order ticket: **Market** (taker swap + slippage + hybrid quote) and **Limit** (resting book),
 * plus cancel + “my placements” ([GitLab #152](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/152)).
 */
export function TradeOrderTicket({
  pairAddr,
  pairs,
  pairsLoading,
  indexerPair,
}: {
  pairAddr: string
  pairs: PairInfo[]
  pairsLoading: boolean
  indexerPair?: IndexerPair | null
}) {
  const address = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)
  const wallet = getConnectedWallet()
  const isWalletConnected = !!address && !!wallet
  const queryClient = useQueryClient()
  const limitPriceInputId = useId()
  const cancelLimitOrderInputId = useId()

  const [side, setSide] = useState<'bid' | 'ask'>('bid')
  const [orderTab, setOrderTab] = useState<'limit' | 'market'>('limit')
  const [price, setPrice] = useState('1')
  const {
    maxSteps,
    setMaxSteps,
    expiresAt,
    setExpiresAt,
    amountHuman,
    setAmountHuman,
    limitAdvancedOpen,
    setLimitAdvancedOpen,
  } = useLimitOrderForm()
  const [cancelOrderId, setCancelOrderId] = useState('')
  const [lastIndexedOrderId, setLastIndexedOrderId] = useState<number | null>(null)

  const selectedPair = useMemo(() => pairs.find((p) => p.contract_addr === pairAddr), [pairs, pairAddr])

  const token0 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[0]) : ''
  const token1 = selectedPair ? assetInfoLabel(selectedPair.asset_infos[1]) : ''
  const escrowToken = side === 'bid' ? token1 : token0
  const escrowDecimals = escrowToken ? getDecimals(tokenAssetInfo(escrowToken)) : 6
  const escrowBalanceQuery = useLimitOrderEscrowBalance(address, escrowToken)
  const nativeUlunaQuery = useNativeUlunaBalance(address)
  const limitPlaceMinUlunaFees = useMemo(() => estimateLimitOrderPlaceSequenceUlunaFeesTotal(), [])

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

  const isPaused = pausedQuery.data?.paused === true

  const { bestBid, bestAsk, isLoading: bestBookLoading } = useTradeBestBookPrices(pairAddr)

  const ustcOracleQuery = useQuery({
    queryKey: ['indexer-oracle-price-ticket'],
    queryFn: getOraclePrice,
    staleTime: 60_000,
  })

  const crossingBlocker = useMemo(
    () => describeLimitCrossingBlocker(side, price, bestBid, bestAsk),
    [side, price, bestBid, bestAsk]
  )

  const inversePriceLine = useMemo(() => inverseLimitPriceHuman(price), [price])
  const usdHintLine = useMemo(
    () =>
      limitPriceUsdHint(
        price,
        indexerPair?.asset_1.symbol,
        ustcOracleQuery.data?.price_usd != null ? String(ustcOracleQuery.data.price_usd) : null
      ),
    [price, indexerPair?.asset_1.symbol, ustcOracleQuery.data?.price_usd]
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

  const placeLimitCombinedOk = placeEscrowGate.canPlaceLimit && placeNativeGasGate.canPlaceLimit && !crossingBlocker
  const placeLimitInlineGate = useMemo(() => {
    if (crossingBlocker) {
      return { canPlaceLimit: false, userMessage: crossingBlocker, tone: 'warning' as const }
    }
    return placeEscrowGate.userMessage ? placeEscrowGate : placeNativeGasGate
  }, [crossingBlocker, placeEscrowGate, placeNativeGasGate])

  const myPlacements = useMemo(() => {
    if (!address || !placementsQuery.data) return []
    return placementsQuery.data.filter((r) => r.owner === address)
  }, [address, placementsQuery.data])

  const parsedCancelOrderId = parseInt(cancelOrderId, 10)
  const cancelIdIndexedAsCancelled =
    Number.isFinite(parsedCancelOrderId) &&
    parsedCancelOrderId >= 1 &&
    orderIdHasIndexedCancellation(cancellationsQuery.data ?? [], parsedCancelOrderId)

  const placeMutation = useMutation({
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
      const raw = toRawAmount(amountHuman, escrowDecimals)
      if (raw === '0') throw new Error('Enter amount')
      await executeTerraContract(address, escrowToken, {
        increase_allowance: { spender: selectedPair.contract_addr, amount: raw },
      })
      return placeLimitOrder(address, escrowToken, selectedPair.contract_addr, raw, side, price, maxSteps, expiresAt)
    },
    onSuccess: async () => {
      sounds.playSuccess()
      setAmountHuman('')
      queryClient.invalidateQueries({ queryKey: ['limitPlacements'] })
      queryClient.invalidateQueries({ queryKey: ['tokenBalance'] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPage', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['tradeBestBook', pairAddr] })
      setLastIndexedOrderId(null)
      const addr = pairAddr
      const walletAddr = address
      if (!addr.startsWith('terra1') || !walletAddr) return
      for (let i = 0; i < 24; i++) {
        await new Promise((r) => setTimeout(r, 500))
        try {
          const rows = await getPairLimitPlacements(addr, { limit: 100 })
          const mine = rows.filter((r) => r.owner === walletAddr)
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

  const cancelMutation = useMutation({
    mutationFn: async () => {
      if (!address) throw new Error('Connect wallet')
      if (!pairAddr.startsWith('terra1')) throw new Error('Select a pair')
      const id = parseInt(cancelOrderId, 10)
      if (!Number.isFinite(id) || id < 1) throw new Error('Invalid order id')
      const cancels = queryClient.getQueryData<IndexerLimitCancellation[]>(['limitCancellations', pairAddr]) ?? []
      if (orderIdHasIndexedCancellation(cancels, id)) {
        throw new Error('This order has already been cancelled.')
      }
      return cancelLimitOrder(address, pairAddr, id)
    },
    onSuccess: () => {
      sounds.playSuccess()
      setCancelOrderId('')
      setLastIndexedOrderId(null)
      queryClient.invalidateQueries({ queryKey: ['limitPlacements'] })
      queryClient.invalidateQueries({ queryKey: ['limitCancellations', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['limitBookPage', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['tradeBestBook', pairAddr] })
    },
    onError: () => sounds.playError(),
  })

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

  if (pairsLoading) {
    return (
      <div className="flex justify-center py-12 card-neo">
        <Spinner />
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-3 h-full min-h-0 overflow-y-auto card-neo !p-4">
      <div>
        <h3 className="text-xs font-semibold uppercase tracking-wide">Order ticket</h3>
        {!pairAddr.startsWith('terra1') && (
          <p className="text-[10px] mt-1" style={{ color: 'var(--ink-dim)' }}>
            Select a trading pair from the bar above.
          </p>
        )}
      </div>

      {selectedPair && (
        <div className="text-[10px] uppercase tracking-wide font-medium" style={{ color: 'var(--ink-dim)' }}>
          Token0: {token0.slice(0, 14)}… · Token1: {token1.slice(0, 14)}…
        </div>
      )}

      {selectedPair && isPaused && (
        <div className="alert-error text-xs space-y-2" role="status">
          <p>
            Pair is paused — swaps, limit place, cancel, and parked-expiry claim are blocked until governance unpauses
            (L6 / GitLab #120).
          </p>
          <a
            className="underline text-[10px]"
            href={`${DOCS_GITLAB_BASE}/contracts-security-audit.md`}
            target="_blank"
            rel="noopener noreferrer"
          >
            L6 (audit)
          </a>
        </div>
      )}

      {selectedPair && pairAddr.startsWith('terra1') && (
        <div className="flex gap-2" role="tablist" aria-label="Order type">
          <button
            type="button"
            role="tab"
            aria-selected={orderTab === 'limit'}
            data-testid="trade-order-tab-limit"
            className={`tab-neo !text-xs !px-3 !py-1.5 ${orderTab === 'limit' ? 'tab-neo-active' : 'tab-neo-inactive'}`}
            onClick={() => {
              sounds.playButtonPress()
              setOrderTab('limit')
            }}
          >
            Limit
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={orderTab === 'market'}
            data-testid="trade-order-tab-market"
            className={`tab-neo !text-xs !px-3 !py-1.5 ${orderTab === 'market' ? 'tab-neo-active' : 'tab-neo-inactive'}`}
            onClick={() => {
              sounds.playButtonPress()
              setOrderTab('market')
            }}
          >
            Market
          </button>
        </div>
      )}

      <div className="space-y-2 border-t border-white/10 pt-3">
        <h3 className="text-[10px] font-semibold uppercase tracking-wide" style={{ color: 'var(--ink-dim)' }}>
          Side
        </h3>
        <div className="flex flex-wrap gap-3 text-xs">
          <label className="flex items-center gap-2 cursor-pointer">
            <input type="radio" name="trade-side" checked={side === 'bid'} onChange={() => setSide('bid')} />
            Bid ({getTokenDisplaySymbol(token1 || 'token1')})
          </label>
          <label className="flex items-center gap-2 cursor-pointer">
            <input type="radio" name="trade-side" checked={side === 'ask'} onChange={() => setSide('ask')} />
            Ask ({getTokenDisplaySymbol(token0 || 'token0')})
          </label>
        </div>
        {selectedPair && pairAddr.startsWith('terra1') && (
          <p className="text-[10px] font-mono leading-snug" style={{ color: 'var(--ink-subtle)' }}>
            Book head — best bid: {bestBookLoading ? '…' : (bestBid ?? '—')} · best ask:{' '}
            {bestBookLoading ? '…' : (bestAsk ?? '—')} ({getTokenDisplaySymbol(token1 || 'q')} per{' '}
            {getTokenDisplaySymbol(token0 || 'b')})
          </p>
        )}
      </div>

      {orderTab === 'market' && selectedPair && (
        <TradeMarketOrderPanel pairAddr={pairAddr} selectedPair={selectedPair} side={side} isPaused={isPaused} />
      )}

      {orderTab === 'limit' && (
        <div className="space-y-3 border-t border-white/10 pt-3">
          <h3 className="text-xs font-semibold uppercase tracking-wide">Place limit</h3>
          <div>
            <label className="label-neo" htmlFor={limitPriceInputId}>
              Price ({getTokenDisplaySymbol(token1 || 'token1')} per {getTokenDisplaySymbol(token0 || 'token0')})
            </label>
            <input
              id={limitPriceInputId}
              className="input-neo w-full font-mono text-sm"
              value={price}
              onChange={(e) => setPrice(e.target.value)}
            />
            {inversePriceLine && (
              <p className="text-[10px] mt-1 font-mono" style={{ color: 'var(--ink-subtle)' }}>
                ≈ {inversePriceLine} {getTokenDisplaySymbol(token0 || 'token0')} per{' '}
                {getTokenDisplaySymbol(token1 || 'token1')}
              </p>
            )}
            {usdHintLine && (
              <p className="text-[10px] mt-0.5" style={{ color: 'var(--ink-dim)' }}>
                {usdHintLine}
              </p>
            )}
          </div>
          <LimitOrderEscrowAmountField
            compact
            escrowLabel={getTokenDisplaySymbol(escrowToken || '—')}
            escrowDecimals={escrowDecimals}
            amountHuman={amountHuman}
            onAmountChange={setAmountHuman}
            balanceQuery={escrowBalanceQuery}
            onMax={setAmountHuman}
            walletConnected={isWalletConnected}
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
          <button
            type="button"
            className="btn-primary btn-cta w-full !text-xs"
            disabled={
              !isWalletConnected || placeMutation.isPending || !selectedPair || isPaused || !placeLimitCombinedOk
            }
            onClick={() => {
              if (!isWalletConnected) openWalletModal()
              else placeMutation.mutate()
            }}
          >
            {!isWalletConnected ? 'Connect Wallet' : placeMutation.isPending ? 'Placing…' : 'Place limit'}
          </button>
          <LimitOrderEscrowPlaceGuardMessage gate={placeLimitInlineGate} data-testid="trade-limit-place-guard" />
          {placeMutation.isError && <TxResultAlert type="error" message={(placeMutation.error as Error).message} />}
          {placeMutation.isSuccess && (
            <TxResultAlert type="success" message="Limit order submitted." txHash={placeMutation.data} />
          )}
          {lastIndexedOrderId != null && (
            <p className="text-[10px] font-mono" data-testid="trade-last-placed-order-id">
              Last indexed: #{lastIndexedOrderId}
            </p>
          )}
        </div>
      )}

      <div className="space-y-3 border-t border-white/10 pt-3">
        <h3 className="text-xs font-semibold uppercase tracking-wide">Cancel limit</h3>
        <div>
          <label className="label-neo" htmlFor={cancelLimitOrderInputId}>
            Order ID
          </label>
          <input
            id={cancelLimitOrderInputId}
            className="input-neo w-full font-mono text-sm"
            value={cancelOrderId}
            onChange={(e) => setCancelOrderId(e.target.value)}
            placeholder="Order ID"
          />
        </div>
        <button
          type="button"
          className="btn-primary btn-cta w-full !text-xs"
          disabled={
            !isWalletConnected || cancelMutation.isPending || !pairAddr || isPaused || cancelIdIndexedAsCancelled
          }
          onClick={() => {
            if (!isWalletConnected) openWalletModal()
            else cancelMutation.mutate()
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

      {pairAddr && address && (
        <LimitOrderMyPlacementsPanel
          variant="compact"
          pairAddr={pairAddr}
          pair={selectedPair}
          walletAddress={address}
          rows={myPlacements}
          isLoading={placementsQuery.isLoading}
          isWalletConnected={isWalletConnected}
          isPairPaused={isPaused}
          openWalletModal={openWalletModal}
        />
      )}
    </div>
  )
}
