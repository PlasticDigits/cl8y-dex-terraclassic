import { useMemo, useState, useEffect, useId } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useWalletStore } from '@/hooks/useWallet'
import { usePairLimitCancellations } from '@/hooks/usePairLimitCancellations'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { placeLimitOrder, cancelLimitOrder, getPairPaused } from '@/services/terraclassic/pair'
import {
  executeTerraContract,
  estimateLimitOrderPlaceSequenceUlunaFeesTotal,
} from '@/services/terraclassic/transactions'
import { getPairLimitBookPage, getPairLimitPlacements, getPair, getTrades } from '@/services/indexer/client'
import { sounds } from '@/lib/sounds'
import { MenuSelect, TxResultAlert, Spinner } from '@/components/ui'
import { assetInfoLabel, tokenAssetInfo, type IndexerLimitCancellation, type IndexerPair } from '@/types'
import { getDecimals, toRawAmount } from '@/utils/formatAmount'
import { evaluateLimitOrderEscrowPlaceGate } from '@/utils/limitOrderEscrowBalanceGate'
import { evaluateLimitOrderNativeGasPlaceGate } from '@/utils/limitOrderNativeGasBalanceGate'
import { warnIndexerPlacementPollFailed } from '@/utils/warnIndexerPlacementPollFailed'
import { pairInfosToMenuSelectOptions } from '@/utils/pairMenuOptions'
import { fetchCW20TokenInfo, getTokenDisplaySymbol, shortenAddress } from '@/utils/tokenDisplay'
import { orderIdHasIndexedCancellation } from '@/utils/limitOrderCancelUserMessage'
import { DOCS_GITLAB_BASE } from '@/utils/constants'
import { useLimitOrderPriceRefBundle } from '@/hooks/useLimitOrderPriceRefBundle'
import { useLimitOrderForm } from '@/hooks/useLimitOrderForm'
import { useLimitOrderEscrowBalance } from '@/hooks/useLimitOrderEscrowBalance'
import { useNativeUlunaBalance } from '@/hooks/useNativeUlunaBalance'
import { LimitOrderAdvancedLimitSettings } from '@/components/trade/LimitOrderAdvancedLimitSettings'
import { LimitOrderBidAskSideSelector } from '@/components/trade/LimitOrderBidAskSideSelector'
import { LimitOrderEscrowAmountField } from '@/components/trade/LimitOrderEscrowAmountField'
import { LimitOrderEscrowPlaceGuardMessage } from '@/components/trade/LimitOrderEscrowPlaceGuardMessage'
import { LimitOrderExpiryField } from '@/components/trade/LimitOrderExpiryField'
import { LimitOrderMyPlacementsPanel } from '@/components/trade/LimitOrderMyPlacementsPanel'
import { evaluateLimitOrderPricePlaceGate } from '@/utils/limitOrderPricePlaceGate'
import { LimitOrderPlaceLimitHeading, LimitOrderPriceInputWithContext } from '@/components/trade/LimitOrderPriceField'
import { WalletIndexerHistoryPanel } from '@/components/trade/WalletIndexerHistoryPanel'

export default function LimitOrdersPage() {
  const address = useWalletStore((s) => s.address)
  const openWalletModal = useWalletStore((s) => s.openWalletModal)
  const wallet = getConnectedWallet()
  const isWalletConnected = !!address && !!wallet
  const queryClient = useQueryClient()
  const limitOrdersPriceInputId = useId()
  const limitOrdersCancelOrderInputId = useId()

  const [pairAddr, setPairAddr] = useState('')
  const [side, setSide] = useState<'bid' | 'ask'>('bid')
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

  const pairsQuery = useQuery({
    queryKey: ['allPairs'],
    queryFn: () => getAllPairsPaginated(),
    staleTime: 60_000,
  })

  const pairs = pairsQuery.data?.pairs ?? []

  const pairMenuOptions = useMemo(() => pairInfosToMenuSelectOptions(pairs, { variant: 'full' }), [pairs])

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

  const indexerPairQuery = useQuery({
    queryKey: ['indexer-pair-limit-orders', pairAddr],
    queryFn: () => getPair(pairAddr),
    enabled: pairAddr.startsWith('terra1'),
    staleTime: 60_000,
    retry: false,
  })

  const tradesForLimitQuery = useQuery({
    queryKey: ['pair-trades-limit-orders', pairAddr],
    queryFn: () => getTrades(pairAddr, 40),
    enabled: pairAddr.startsWith('terra1'),
    refetchInterval: 15_000,
    retry: false,
  })

  const indexerPair: IndexerPair | undefined = indexerPairQuery.data
  const latestTrade = tradesForLimitQuery.data?.[0]
  const tapeHeadlineUsd = latestTrade?.price

  const { refToken1PerToken0, refSource, refResolutionLoading, refResolutionError } = useLimitOrderPriceRefBundle({
    pairAddr,
    selectedPair,
    indexerPair,
    latestTrade,
  })

  const bookBidQuery = useQuery({
    queryKey: ['limitBookPagePreview', pairAddr, 'bid'],
    queryFn: () => getPairLimitBookPage(pairAddr, 'bid', { limit: 20 }),
    enabled: pairAddr.startsWith('terra1'),
    staleTime: 10_000,
  })

  const bookAskQuery = useQuery({
    queryKey: ['limitBookPagePreview', pairAddr, 'ask'],
    queryFn: () => getPairLimitBookPage(pairAddr, 'ask', { limit: 20 }),
    enabled: pairAddr.startsWith('terra1'),
    staleTime: 10_000,
  })

  const isPaused = pausedQuery.data?.paused === true

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

  const placePriceGate = useMemo(
    () =>
      evaluateLimitOrderPricePlaceGate(side, price, refToken1PerToken0, {
        refResolutionLoading,
        refResolutionError,
      }),
    [side, price, refToken1PerToken0, refResolutionLoading, refResolutionError]
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
    !expiryPastBlocker

  const placeLimitInlineGate = expiryPastBlocker
    ? { canPlaceLimit: false, userMessage: expiryPastBlocker, tone: 'warning' as const }
    : !placePriceGate.canPlaceLimit
      ? placePriceGate
      : placeEscrowGate.userMessage
        ? placeEscrowGate
        : placeNativeGasGate

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
      queryClient.invalidateQueries({ queryKey: ['limitBookPagePreview', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['limitOrderPricePoolRef', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['wallet-indexer-history'] })
      setLastIndexedOrderId(null)
      const addr = pairAddr
      const wallet = address
      if (!addr.startsWith('terra1') || !wallet) return
      for (let i = 0; i < 24; i++) {
        await new Promise((r) => setTimeout(r, 500))
        try {
          const rows = await getPairLimitPlacements(addr, { limit: 100 })
          const mine = rows.filter((r) => r.owner === wallet)
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
      queryClient.invalidateQueries({ queryKey: ['limitBookPagePreview', pairAddr] })
      queryClient.invalidateQueries({ queryKey: ['wallet-indexer-history'] })
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

  return (
    <div className="max-w-[560px] mx-auto">
      <div className="relative">
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-x-6 top-8 h-[78%] rounded-[28px] theme-hero-glow blur-2xl"
        />
        <div className="shell-panel-strong relative z-10">
          <div className="mb-6">
            <h1 className="text-lg font-semibold uppercase tracking-wide font-heading">Limit Orders</h1>
            <p className="text-sm mt-1" style={{ color: 'var(--ink-dim)' }}>
              Place or cancel on-chain limits on a pair. Bids escrow token1; asks escrow token0 (pair ordering).
            </p>
          </div>

          {pairsQuery.isLoading && (
            <div className="flex items-center gap-2 py-8 justify-center">
              <Spinner />
            </div>
          )}

          {!pairsQuery.isLoading && (
            <div className="space-y-6">
              <div>
                <label className="label-neo" htmlFor="limit-pair">
                  Pair
                </label>
                <MenuSelect
                  id="limit-pair"
                  className="relative w-full"
                  aria-label="Trading pair"
                  value={pairAddr}
                  options={pairMenuOptions}
                  emptyLabel="No pairs on factory"
                  onChange={setPairAddr}
                />
              </div>

              {selectedPair && (
                <div className="text-xs uppercase tracking-wide font-medium" style={{ color: 'var(--ink-dim)' }}>
                  Token0: {shortenAddress(token0)} · Token1: {shortenAddress(token1)}
                </div>
              )}

              {selectedPair && isPaused && (
                <div className="alert-error text-sm space-y-2" role="status">
                  <p>
                    This pair is paused by governance. New limit orders, cancel, and parked-expiry Claim refund are
                    unavailable until the pair is unpaused (invariant L6 / GitLab #120 — all maker withdrawals frozen).
                    Escrow remains in the pair contract until unpause.
                  </p>
                  <p className="text-xs opacity-90">
                    <a
                      className="underline hover:opacity-80"
                      href={`${DOCS_GITLAB_BASE}/contracts-security-audit.md`}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      Security audit (L6)
                    </a>
                    {' · '}
                    <a
                      className="underline hover:opacity-80"
                      href={`${DOCS_GITLAB_BASE}/limit-orders.md`}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      limit-orders.md
                    </a>
                  </p>
                </div>
              )}

              {selectedPair && (
                <div className="card-neo !p-4 space-y-3">
                  <h2 className="text-sm font-semibold uppercase tracking-wide">On-chain order book (indexer → LCD)</h2>
                  {(bookBidQuery.isLoading || bookAskQuery.isLoading) && <Spinner />}
                  {(bookBidQuery.isError || bookAskQuery.isError) && (
                    <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
                      Book unavailable (indexer or LCD).
                    </p>
                  )}
                  {!bookBidQuery.isLoading && !bookBidQuery.isError && bookBidQuery.data && (
                    <div>
                      <div
                        className="text-xs font-medium uppercase tracking-wide mb-1"
                        style={{ color: 'var(--ink-dim)' }}
                      >
                        Bids
                      </div>
                      <ul className="text-xs font-mono space-y-1 max-h-32 overflow-y-auto">
                        {bookBidQuery.data.orders.length === 0 && <li className="opacity-70">(empty)</li>}
                        {bookBidQuery.data.orders.map((o) => (
                          <li key={`bid-${o.order_id}`}>
                            #{o.order_id} · {o.price} · rem {o.remaining}
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                  {!bookAskQuery.isLoading && !bookAskQuery.isError && bookAskQuery.data && (
                    <div>
                      <div
                        className="text-xs font-medium uppercase tracking-wide mb-1"
                        style={{ color: 'var(--ink-dim)' }}
                      >
                        Asks
                      </div>
                      <ul className="text-xs font-mono space-y-1 max-h-32 overflow-y-auto">
                        {bookAskQuery.data.orders.length === 0 && <li className="opacity-70">(empty)</li>}
                        {bookAskQuery.data.orders.map((o) => (
                          <li key={`ask-${o.order_id}`}>
                            #{o.order_id} · {o.price} · rem {o.remaining}
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                </div>
              )}

              <div className="card-neo !p-4 space-y-4">
                <LimitOrderPlaceLimitHeading />
                <LimitOrderBidAskSideSelector
                  idPrefix="limit-orders"
                  side={side}
                  onSideChange={setSide}
                  bidLabel={`Bid (escrow ${getTokenDisplaySymbol(token1 || 'token1')})`}
                  askLabel={`Ask (escrow ${getTokenDisplaySymbol(token0 || 'token0')})`}
                />
                <LimitOrderPriceInputWithContext
                  side={side}
                  price={price}
                  onPriceChange={setPrice}
                  inputId={limitOrdersPriceInputId}
                  refToken1PerToken0={refToken1PerToken0}
                  refSource={refSource}
                  tapeHeadlineUsd={tapeHeadlineUsd}
                  token0Label={getTokenDisplaySymbol(token0 || 'token0')}
                  token1Label={getTokenDisplaySymbol(token1 || 'token1')}
                />
                <LimitOrderEscrowAmountField
                  escrowLabel={getTokenDisplaySymbol(escrowToken || '—')}
                  escrowDecimals={escrowDecimals}
                  amountHuman={amountHuman}
                  onAmountChange={setAmountHuman}
                  balanceQuery={escrowBalanceQuery}
                  onMax={setAmountHuman}
                  walletConnected={isWalletConnected}
                />
                <LimitOrderExpiryField value={expiresAt} onChange={setExpiresAt} idPrefix="limit-orders-page" />
                <LimitOrderAdvancedLimitSettings
                  open={limitAdvancedOpen}
                  onOpenChange={setLimitAdvancedOpen}
                  maxSteps={maxSteps}
                  onMaxStepsChange={setMaxSteps}
                  expiresAt={expiresAt}
                  onExpiresAtChange={setExpiresAt}
                  idPrefix="limit-orders-page"
                />
                <button
                  type="button"
                  className="btn-primary btn-cta w-full"
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
                <LimitOrderEscrowPlaceGuardMessage gate={placeLimitInlineGate} data-testid="limits-page-place-guard" />
                {placeMutation.isError && (
                  <TxResultAlert type="error" message={(placeMutation.error as Error).message} />
                )}
                {placeMutation.isSuccess && (
                  <TxResultAlert type="success" message="Limit order submitted." txHash={placeMutation.data} />
                )}
                {lastIndexedOrderId != null && (
                  <p className="text-xs font-mono" data-testid="last-placed-order-id">
                    Last indexed placement for your wallet: order #{lastIndexedOrderId}
                  </p>
                )}
              </div>

              <div className="card-neo !p-4 space-y-4">
                <h2 className="text-sm font-semibold uppercase tracking-wide">Cancel limit</h2>
                <div>
                  <label className="label-neo" htmlFor={limitOrdersCancelOrderInputId}>
                    Order ID
                  </label>
                  <input
                    id={limitOrdersCancelOrderInputId}
                    className="input-neo w-full font-mono"
                    value={cancelOrderId}
                    onChange={(e) => setCancelOrderId(e.target.value)}
                    placeholder="e.g. 42"
                  />
                </div>
                <button
                  type="button"
                  className="btn-primary btn-cta w-full"
                  disabled={
                    !isWalletConnected ||
                    cancelMutation.isPending ||
                    !pairAddr ||
                    isPaused ||
                    cancelIdIndexedAsCancelled
                  }
                  onClick={() => {
                    if (!isWalletConnected) openWalletModal()
                    else cancelMutation.mutate()
                  }}
                >
                  {!isWalletConnected ? 'Connect Wallet' : cancelMutation.isPending ? 'Cancelling…' : 'Cancel limit'}
                </button>
                {cancelIdIndexedAsCancelled && (
                  <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
                    This order id already has an indexed cancellation.
                  </p>
                )}
                {cancelMutation.isError && (
                  <TxResultAlert type="error" message={(cancelMutation.error as Error).message} />
                )}
                {cancelMutation.isSuccess && (
                  <TxResultAlert type="success" message="Cancel transaction submitted." txHash={cancelMutation.data} />
                )}
              </div>

              {pairAddr && address && (
                <div className="card-neo !p-4 space-y-2">
                  <LimitOrderMyPlacementsPanel
                    variant="page"
                    pairAddr={pairAddr}
                    pair={selectedPair}
                    walletAddress={address}
                    rows={myPlacements}
                    isLoading={placementsQuery.isLoading}
                    isWalletConnected={isWalletConnected}
                    isPairPaused={isPaused}
                    openWalletModal={openWalletModal}
                  />
                </div>
              )}

              {pairAddr && address && <WalletIndexerHistoryPanel walletAddress={address} pairAddress={pairAddr} />}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
