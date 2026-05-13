import { useMemo, useState, useEffect, useId } from 'react'
import type { ReactNode } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { usePairLimitCancellations } from '@/hooks/usePairLimitCancellations'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { placeLimitOrder, cancelLimitOrder, getPairPaused } from '@/services/terraclassic/pair'
import {
  executeTerraContract,
  estimateLimitOrderPlaceSequenceUlunaFeesTotal,
} from '@/services/terraclassic/transactions'
import { getPairLimitPlacements } from '@/services/indexer/client'
import { sounds } from '@/lib/sounds'
import { TxResultAlert, Spinner } from '@/components/ui'
import {
  assetInfoLabel,
  tokenAssetInfo,
  type IndexerLimitCancellation,
  type IndexerPair,
  type IndexerTrade,
  type PairInfo,
} from '@/types'
import { getDecimals, toRawAmount } from '@/utils/formatAmount'
import { evaluateLimitOrderEscrowPlaceGate } from '@/utils/limitOrderEscrowBalanceGate'
import { evaluateLimitOrderNativeGasPlaceGate } from '@/utils/limitOrderNativeGasBalanceGate'
import { evaluateLimitOrderPricePlaceGate } from '@/utils/limitOrderPricePlaceGate'
import { useLimitOrderPriceRefBundle } from '@/hooks/useLimitOrderPriceRefBundle'
import { warnIndexerPlacementPollFailed } from '@/utils/warnIndexerPlacementPollFailed'
import { orderIdHasIndexedCancellation } from '@/utils/limitOrderCancelUserMessage'
import { fetchCW20TokenInfo, getTokenDisplaySymbol } from '@/utils/tokenDisplay'
import { DOCS_GITLAB_BASE } from '@/utils/constants'
import { useLimitOrderForm } from '@/hooks/useLimitOrderForm'
import { useLimitOrderEscrowBalance } from '@/hooks/useLimitOrderEscrowBalance'
import { useNativeUlunaBalance } from '@/hooks/useNativeUlunaBalance'
import { LimitOrderAdvancedLimitSettings } from '@/components/trade/LimitOrderAdvancedLimitSettings'
import { LimitOrderBidAskSideSelector } from '@/components/trade/LimitOrderBidAskSideSelector'
import { LimitOrderEscrowAmountField } from '@/components/trade/LimitOrderEscrowAmountField'
import { LimitOrderEscrowPlaceGuardMessage } from '@/components/trade/LimitOrderEscrowPlaceGuardMessage'
import { LimitOrderExpiryField } from '@/components/trade/LimitOrderExpiryField'
import { LimitOrderMyPlacementsPanel } from '@/components/trade/LimitOrderMyPlacementsPanel'
import { LimitOrderPlaceLimitHeading, LimitOrderPriceInputWithContext } from '@/components/trade/LimitOrderPriceField'
import { useTradeBestBookPrices } from '@/hooks/useTradeBestBookPrices'
import { describeLimitCrossingBlocker } from '@/utils/limitOrderNonCrossing'
import { TradeMarketOrderPanel } from '@/components/trade/TradeMarketOrderPanel'

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

/**
 * Trade workspace order ticket: **Market** (taker swap + slippage + hybrid quote) and **Limit** (resting book),
 * plus cancel + “my placements” ([GitLab #152](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/152)).
 */
export function TradeOrderTicket({
  pairAddr,
  pairs,
  pairsLoading,
  indexerPair,
  latestTrade,
  tapeHeadlineUsd,
}: {
  pairAddr: string
  pairs: PairInfo[]
  pairsLoading: boolean
  indexerPair?: IndexerPair | null
  latestTrade?: IndexerTrade | null
  tapeHeadlineUsd?: string | null
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
    !expiryPastBlocker

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
      const priceGate = evaluateLimitOrderPricePlaceGate(side, price, refToken1PerToken0, {
        refResolutionLoading,
        refResolutionError,
      })
      if (!priceGate.canPlaceLimit) {
        throw new Error(priceGate.userMessage ?? 'Invalid limit price for this side.')
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
      queryClient.invalidateQueries({ queryKey: ['limitOrderPricePoolRef', pairAddr] })
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

  const token0Display = indexerPair?.asset_0.symbol ?? getTokenDisplaySymbol(token0 || 'token0')
  const token1Display = indexerPair?.asset_1.symbol ?? getTokenDisplaySymbol(token1 || 'token1')
  const sideAction =
    side === 'bid'
      ? { verb: 'Buy', receive: token0Display, pay: token1Display, tone: 'bid' as const }
      : { verb: 'Sell', receive: token1Display, pay: token0Display, tone: 'ask' as const }
  const walletLabel = isWalletConnected && address ? `${address.slice(0, 8)}…${address.slice(-6)}` : 'Connect wallet'
  const bestBidLabel = bestBookLoading ? '…' : (bestBid ?? '—')
  const bestAskLabel = bestBookLoading ? '…' : (bestAsk ?? '—')

  return (
    <div className="flex flex-col h-full min-h-0 overflow-y-auto card-neo !p-0">
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
            <p className="mt-1 text-[10px] leading-snug" style={{ color: 'var(--ink-dim)' }}>
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

      <div className="flex flex-col gap-3 p-4">
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
          <div
            className="grid grid-cols-2 gap-1 rounded-2xl border border-white/10 p-1"
            style={{ background: 'rgba(255,255,255,0.025)' }}
            role="tablist"
            aria-label="Order type"
          >
            <button
              type="button"
              role="tab"
              aria-selected={orderTab === 'limit'}
              data-testid="trade-order-tab-limit"
              className={`tab-neo !text-xs !px-3 !py-2 w-full justify-center ${orderTab === 'limit' ? 'tab-neo-active' : 'tab-neo-inactive'}`}
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
              className={`tab-neo !text-xs !px-3 !py-2 w-full justify-center ${orderTab === 'market' ? 'tab-neo-active' : 'tab-neo-inactive'}`}
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
            onSideChange={setSide}
            bidLabel={`Buy ${token0Display}`}
            askLabel={`Sell ${token0Display}`}
          />
          {selectedPair && pairAddr.startsWith('terra1') && (
            <div className="grid grid-cols-2 gap-2">
              <TicketStat label="Best bid" value={bestBidLabel} tone="bid" />
              <TicketStat label="Best ask" value={bestAskLabel} tone="ask" />
            </div>
          )}
          <p className="text-[10px] leading-snug" style={{ color: 'var(--ink-dim)' }}>
            Prices are quoted in {token1Display} per {token0Display}. Buy limits should sit below the reference; sell
            limits above it.
          </p>
        </TicketSection>

        {orderTab === 'market' && selectedPair && (
          <TicketSection eyebrow="Take liquidity" title={`${sideAction.verb} now`} tone="action">
            <TradeMarketOrderPanel pairAddr={pairAddr} selectedPair={selectedPair} side={side} isPaused={isPaused} />
          </TicketSection>
        )}

        {orderTab === 'limit' && (
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
              data-testid="trade-limit-submit"
              className="btn-primary btn-cta w-full !text-xs"
              disabled={
                placeMutation.isPending || !selectedPair || isPaused || (isWalletConnected && !placeLimitCombinedOk)
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
          </TicketSection>
        )}

        <TicketSection eyebrow="Manage" title="Cancel resting limit" tone="manage">
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
            data-testid="trade-cancel-submit"
            className="btn-primary btn-cta w-full !text-xs"
            disabled={
              cancelMutation.isPending || !pairAddr || isPaused || (isWalletConnected && cancelIdIndexedAsCancelled)
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
        </TicketSection>

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
    </div>
  )
}
