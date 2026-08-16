import { useQuery } from '@tanstack/react-query'
import type { UseMutationResult } from '@tanstack/react-query'
import { getPairLimitPlacements } from '@/services/indexer/client'
import { useLimitBookInfinite } from '@/hooks/useLimitBookInfinite'
import { usePairLimitCancellations } from '@/hooks/usePairLimitCancellations'
import { useRecentlyCancelledOrderIds } from '@/hooks/useLimitOrderStatuses'
import type { LimitOrderCancelInput } from '@/hooks/useLimitOrderCancelMutation'
import { Spinner } from '@/components/ui'
import type { IndexerPair, IndexerShallowLimitOrder, PairInfo } from '@/types'
import type { LimitBookTicketDraft } from '@/types/limitBookTicketDraft'
import { formatNum, formatTokenAmount, fromRawAmount } from '@/utils/formatAmount'
import { orderIdHasIndexedCancellation } from '@/utils/limitOrderCancelUserMessage'
import { partitionLimitPlacementsByLifecycle } from '@/utils/limitPlacementLifecycle'
import { TRADE_PANEL_BOOK_UNAVAILABLE } from '@/utils/indexerTradeOutageCopy'
import { flattenLimitBookPages } from '@/utils/limitBookInsertHint'
import { TradeMarketDataUnavailableNotice } from '@/components/trade/TradeMarketDataUnavailableNotice'
import { factoryToken1PerToken0ToDisplayPrice } from '@/utils/tradePairDisplayOrientation'
import { limitPriceDecimalsFromPair, scaleRawLimitPriceForDisplay } from '@/utils/limitOrderPriceScale'

function rawTotal(orders: IndexerShallowLimitOrder[]): bigint {
  return orders.reduce((acc, order) => {
    try {
      return acc + BigInt(order.remaining)
    } catch {
      return acc
    }
  }, 0n)
}

function formatBookPrice(
  raw: string,
  inverted = false,
  scale?: { decimals0: number; decimals1: number } | null
): string {
  const human = scaleRawLimitPriceForDisplay(raw, scale)
  const shown = inverted ? (factoryToken1PerToken0ToDisplayPrice(human, true) ?? human) : human
  return formatNum(shown, 7)
    .replace(/(\.\d*?[1-9])0+$/, '$1')
    .replace(/\.0+$/, '')
}

function normalizeBookSide(raw: string | undefined): 'bid' | 'ask' | null {
  const s = raw?.trim().toLowerCase()
  if (s === 'bid' || s === 'ask') return s
  return null
}

function BookRow({
  order,
  side,
  sizeDecimals,
  maxRaw,
  cumulative,
  walletAddress,
  isPairPaused,
  isWalletConnected,
  openWalletModal,
  cancelMutation,
  onPrefillLimitTicket,
  cancellations,
  hintAfterOrderId,
  displayInverted,
  limitPriceScale,
}: {
  order: IndexerShallowLimitOrder
  side: 'bid' | 'ask'
  sizeDecimals: number
  maxRaw: bigint
  cumulative: string
  walletAddress?: string
  isPairPaused: boolean
  isWalletConnected: boolean
  openWalletModal?: () => void
  cancelMutation?: UseMutationResult<string, Error, LimitOrderCancelInput, unknown>
  onPrefillLimitTicket?: (draft: LimitBookTicketDraft) => void
  cancellations: { order_id: number }[]
  hintAfterOrderId?: number | null
  displayInverted?: boolean
  limitPriceScale?: { decimals0: number; decimals1: number } | null
}) {
  let remainingRaw = 0n
  let cumulativeRaw = 0n
  try {
    remainingRaw = BigInt(order.remaining)
  } catch {
    remainingRaw = 0n
  }
  try {
    cumulativeRaw = BigInt(cumulative)
  } catch {
    cumulativeRaw = remainingRaw
  }
  const depthPct = maxRaw > 0n ? Number((cumulativeRaw * 100n) / maxRaw) : 0
  const sideColor = side === 'bid' ? 'var(--color-positive)' : 'var(--color-negative)'
  const sideBg = side === 'bid' ? 'rgba(34, 197, 94, 0.13)' : 'rgba(239, 68, 68, 0.13)'

  const isMine = !!walletAddress && order.owner === walletAddress
  const normalized = normalizeBookSide(order.side)
  const rowSide = normalized ?? side
  const alreadyCancelled = orderIdHasIndexedCancellation(cancellations, order.order_id)
  const pendingThis = cancelMutation?.isPending && cancelMutation.variables === order.order_id
  const canUseRowActions = !!cancelMutation && !!onPrefillLimitTicket

  const onCancelClick = () => {
    if (!isWalletConnected) {
      openWalletModal?.()
      return
    }
    if (isPairPaused || alreadyCancelled || !cancelMutation) return
    const ok = window.confirm(
      `Cancel order #${order.order_id}? Funds return to your wallet after the transaction confirms.`
    )
    if (!ok) return
    cancelMutation.mutate(order.order_id)
  }

  const onEditClick = () => {
    if (!isWalletConnected) {
      openWalletModal?.()
      return
    }
    if (!onPrefillLimitTicket) return
    const amountHuman = fromRawAmount(order.remaining, sizeDecimals)
    onPrefillLimitTicket({
      side: rowSide,
      price: scaleRawLimitPriceForDisplay(order.price, limitPriceScale),
      amountHuman,
      orderId: order.order_id,
      expiresAt: order.expires_at ?? null,
      hintAfterOrderId: hintAfterOrderId ?? null,
    })
  }

  const rowLabel = `${rowSide} order ${order.order_id}, price ${formatBookPrice(order.price, displayInverted, limitPriceScale)}, size ${formatTokenAmount(order.remaining, sizeDecimals, 4)}, cumulative ${formatTokenAmount(cumulative, sizeDecimals, 4)}`

  return (
    <tr
      className="relative [&>td]:align-middle"
      title={`Order #${order.order_id} · owner ${order.owner.slice(0, 12)}… · raw remaining ${order.remaining}`}
      aria-label={rowLabel}
    >
      <td className="relative rounded-l-lg px-2 py-1.5 text-[11px] font-mono tabular-nums overflow-hidden">
        <span
          className="absolute inset-y-0 right-0 pointer-events-none"
          aria-hidden="true"
          style={{ width: `${Math.min(depthPct, 100)}%`, background: sideBg }}
        />
        <div className="relative min-w-0">
          <span className="block text-[9px] leading-tight" style={{ color: 'var(--ink-subtle)' }}>
            #{order.order_id}
          </span>
          <span className="font-semibold leading-tight" style={{ color: sideColor }}>
            {formatBookPrice(order.price, displayInverted, limitPriceScale)}
          </span>
        </div>
      </td>
      <td
        className="relative px-2 py-1.5 text-right text-[11px] font-mono tabular-nums"
        style={{ color: 'var(--ink)' }}
      >
        {formatTokenAmount(order.remaining, sizeDecimals, 4)}
      </td>
      <td
        className="relative px-2 py-1.5 text-right text-[11px] font-mono tabular-nums"
        style={{ color: 'var(--ink-dim)' }}
      >
        {formatTokenAmount(cumulative, sizeDecimals, 4)}
      </td>
      <td className="relative rounded-r-lg px-2 py-1.5">
        <div className="flex items-center justify-end gap-0.5 shrink-0">
          {isMine && canUseRowActions && (
            <>
              <button
                type="button"
                data-testid={`trade-book-edit-${side}-${order.order_id}`}
                className="rounded-md border border-white/15 px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wide hover:bg-white/10 disabled:opacity-40"
                style={{ color: 'var(--ink)' }}
                disabled={isPairPaused || pendingThis}
                title="Load this order into the limit ticket — change price only to update in one tx, or cancel first to replace size/side/expiry."
                aria-label={`Edit order ${order.order_id} — load into limit ticket for price update or replace`}
                onClick={onEditClick}
              >
                Edit
              </button>
              <button
                type="button"
                data-testid={`trade-book-cancel-${side}-${order.order_id}`}
                className="rounded-md border border-white/15 px-1.5 py-0.5 text-[10px] font-semibold leading-none hover:bg-red-500/15 disabled:opacity-40"
                style={{ color: 'var(--color-negative)' }}
                disabled={isPairPaused || alreadyCancelled || pendingThis}
                title={alreadyCancelled ? 'Already cancelled' : 'Cancel this resting order'}
                aria-label={
                  alreadyCancelled ? `Order ${order.order_id} already cancelled` : `Cancel order ${order.order_id}`
                }
                onClick={onCancelClick}
              >
                {alreadyCancelled ? '—' : '×'}
              </button>
            </>
          )}
        </div>
      </td>
    </tr>
  )
}

function BookSideColumn({
  title,
  pairAddress,
  side,
  priceLabel,
  sizeLabel,
  sizeDecimals,
  walletAddress,
  isPairPaused,
  isWalletConnected,
  openWalletModal,
  cancelMutation,
  onPrefillLimitTicket,
  displayInverted,
  limitPriceScale,
}: {
  title: string
  pairAddress: string
  side: 'bid' | 'ask'
  priceLabel: string
  sizeLabel: string
  sizeDecimals: number
  walletAddress?: string
  isPairPaused: boolean
  isWalletConnected: boolean
  openWalletModal?: () => void
  cancelMutation?: UseMutationResult<string, Error, LimitOrderCancelInput, unknown>
  onPrefillLimitTicket?: (draft: LimitBookTicketDraft) => void
  displayInverted?: boolean
  limitPriceScale?: { decimals0: number; decimals1: number } | null
}) {
  const cancellationsQuery = usePairLimitCancellations(pairAddress)

  const q = useLimitBookInfinite(pairAddress, side)

  const orders = flattenLimitBookPages(q.data?.pages).orders
  const maxRaw = rawTotal(orders)
  let runningRaw = 0n
  const rows = orders.map((order) => {
    try {
      runningRaw += BigInt(order.remaining)
    } catch {
      // Keep the running total stable when a malformed row arrives.
    }
    return { order, cumulative: runningRaw.toString() }
  })
  const sideTone = side === 'bid' ? 'var(--color-positive)' : 'var(--color-negative)'
  const emptyLabel = side === 'bid' ? 'No resting bids yet' : 'No resting asks yet'
  const cancellations = cancellationsQuery.data ?? []

  return (
    <div className="flex flex-col min-h-0 h-full card-glass !p-3 overflow-hidden">
      <div className="flex items-center justify-between gap-2 mb-2 shrink-0">
        <div className="flex items-center gap-2 min-w-0">
          <span
            className="h-2 w-2 rounded-full shadow-[0_0_12px_currentColor]"
            style={{ color: sideTone, background: sideTone }}
          />
          <div className="min-w-0">
            <div className="text-xs font-semibold uppercase tracking-wide" style={{ color: 'var(--ink)' }}>
              {title}
            </div>
            <div className="text-[9px] uppercase tracking-wide truncate" style={{ color: 'var(--ink-subtle)' }}>
              Open limit orders
            </div>
          </div>
        </div>
        <div
          className="rounded-full border px-2 py-0.5 text-[9px] font-semibold uppercase tracking-wide"
          style={{ color: sideTone, borderColor: 'color-mix(in srgb, currentColor 35%, transparent)' }}
        >
          {orders.length}
        </div>
      </div>
      {q.isLoading && (
        <div className="flex flex-1 items-center justify-center py-6" style={{ color: 'var(--ink-subtle)' }}>
          <Spinner />
        </div>
      )}
      {q.isError && (
        <TradeMarketDataUnavailableNotice
          message={TRADE_PANEL_BOOK_UNAVAILABLE}
          data-testid={`trade-book-unavailable-${side}`}
        />
      )}
      {!q.isLoading && !q.isError && (
        <>
          <div
            className="flex-1 overflow-y-auto min-h-[96px] pr-1"
            tabIndex={0}
            role="region"
            aria-label={`${title} limit orders scroll`}
          >
            <table className="w-full border-separate border-spacing-y-1" aria-label={`${title} limit orders`}>
              <thead>
                <tr className="text-[9px] font-semibold uppercase tracking-wide" style={{ color: 'var(--ink-subtle)' }}>
                  <th scope="col" className="px-2 pb-1 text-left font-semibold">
                    Order / price {priceLabel}
                  </th>
                  <th scope="col" className="px-2 pb-1 text-right font-semibold">
                    Size {sizeLabel}
                  </th>
                  <th scope="col" className="px-2 pb-1 text-right font-semibold">
                    Total
                  </th>
                  <th scope="col" className="px-2 pb-1 text-right font-semibold">
                    <span className="sr-only">Actions</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {orders.length === 0 && (
                  <tr>
                    <td
                      colSpan={4}
                      className="rounded-xl border border-dashed px-3 py-8 text-center text-[11px]"
                      style={{ borderColor: 'var(--line)', color: 'var(--ink-subtle)' }}
                    >
                      {emptyLabel}
                    </td>
                  </tr>
                )}
                {rows.map(({ order, cumulative }, index) => (
                  <BookRow
                    key={`${side}-${order.order_id}`}
                    order={order}
                    side={side}
                    sizeDecimals={sizeDecimals}
                    maxRaw={maxRaw}
                    cumulative={cumulative}
                    walletAddress={walletAddress}
                    isPairPaused={isPairPaused}
                    isWalletConnected={isWalletConnected}
                    openWalletModal={openWalletModal}
                    cancelMutation={cancelMutation}
                    onPrefillLimitTicket={onPrefillLimitTicket}
                    cancellations={cancellations}
                    hintAfterOrderId={index > 0 ? orders[index - 1]?.order_id : null}
                    displayInverted={displayInverted}
                    limitPriceScale={limitPriceScale}
                  />
                ))}
              </tbody>
            </table>
          </div>
          {q.hasNextPage && (
            <button
              type="button"
              className="btn-primary btn-cta !text-[10px] !py-1 !px-2 mt-2 w-full shrink-0"
              disabled={q.isFetchingNextPage}
              onClick={() => void q.fetchNextPage()}
            >
              {q.isFetchingNextPage ? 'Loading…' : 'Load more depth'}
            </button>
          )}
        </>
      )}
    </div>
  )
}

export type OrderBookPanelProps = {
  pairAddress: string
  pair?: IndexerPair
  /** Connected wallet bech32 — enables row actions for matching `owner` rows. */
  walletAddress?: string
  isWalletConnected?: boolean
  isPairPaused?: boolean
  openWalletModal?: () => void
  /** Shared with `TradeOrderTicket` on `/trade` so book rows and ticket use one cancel mutation. */
  cancelLimitOrderMutation?: UseMutationResult<string, Error, LimitOrderCancelInput, unknown>
  /** Opens the limit ticket with side / price / size prefilled (replace-style flow; GitLab #162). */
  onPrefillLimitTicket?: (draft: LimitBookTicketDraft) => void
  /** Factory pair (asset_infos) for cancel-all placement partition; optional if cancel-all unused. */
  factoryPair?: PairInfo
  displayInverted?: boolean
  displayBaseSymbol?: string
  displayQuoteSymbol?: string
}

export function OrderBookPanel({
  pairAddress,
  pair,
  walletAddress,
  isWalletConnected = false,
  isPairPaused = false,
  openWalletModal,
  cancelLimitOrderMutation,
  onPrefillLimitTicket,
  factoryPair,
  displayInverted = false,
  displayBaseSymbol,
  displayQuoteSymbol,
}: OrderBookPanelProps) {
  const placementsQuery = useQuery({
    queryKey: ['limitPlacements', pairAddress],
    queryFn: () => getPairLimitPlacements(pairAddress, { limit: 100 }),
    enabled: !!walletAddress && pairAddress.startsWith('terra1') && !!cancelLimitOrderMutation && !!factoryPair,
    staleTime: 10_000,
  })

  const recentlyCancelledOrderIds = useRecentlyCancelledOrderIds(pairAddress)

  const myActiveOrderIds = (() => {
    if (!walletAddress || !placementsQuery.data || !factoryPair) return []
    const mine = placementsQuery.data.filter((r) => r.owner === walletAddress)
    const { active } = partitionLimitPlacementsByLifecycle(mine)
    return active.map((r) => r.order_id).filter((id) => !recentlyCancelledOrderIds.includes(id))
  })()

  const cancelAllDisabled =
    !walletAddress ||
    !isWalletConnected ||
    isPairPaused ||
    !cancelLimitOrderMutation ||
    cancelLimitOrderMutation.isPending ||
    myActiveOrderIds.length === 0

  const onCancelAllMyResting = async () => {
    if (!walletAddress || !cancelLimitOrderMutation || myActiveOrderIds.length === 0) return
    const ok = window.confirm(
      myActiveOrderIds.length === 1
        ? 'Cancel 1 resting limit order for this pair from your wallet?'
        : `Cancel all ${myActiveOrderIds.length} resting limit orders for this pair in one on-chain transaction?`
    )
    if (!ok) return
    try {
      await cancelLimitOrderMutation.mutateAsync(myActiveOrderIds)
    } catch (e) {
      window.alert((e as Error)?.message ?? String(e))
    }
  }

  if (!pairAddress.startsWith('terra1')) {
    return (
      <div className="card-glass !p-4 text-sm" style={{ color: 'var(--ink-dim)' }}>
        Select a pair to view the order book.
      </div>
    )
  }

  const baseSymbol = displayBaseSymbol ?? pair?.asset_0.symbol ?? 'Base'
  const quoteSymbol = displayQuoteSymbol ?? pair?.asset_1.symbol ?? 'Quote'
  const priceLabel = `${quoteSymbol}/${baseSymbol}`

  return (
    <div className="flex flex-col gap-2 h-full min-h-0">
      <div className="shrink-0 space-y-1">
        <div className="flex flex-wrap items-end justify-between gap-2">
          <div>
            <h2 className="text-xs font-semibold uppercase tracking-wide" style={{ color: 'var(--ink)' }}>
              Order book
            </h2>
            <p
              className="text-[10px] leading-snug max-w-md"
              style={{ color: 'var(--ink-dim)' }}
              title="Your resting limits — edit or cancel on your rows."
            >
              Your orders
            </p>
          </div>
          <div className="flex flex-col items-end gap-1">
            <div
              className="rounded-full border px-2 py-0.5 text-[9px] font-semibold uppercase tracking-wide"
              style={{ borderColor: 'var(--line)', color: 'var(--ink-subtle)' }}
            >
              {priceLabel}
            </div>
            {cancelLimitOrderMutation && onPrefillLimitTicket && (
              <button
                type="button"
                data-testid="trade-book-cancel-all-mine"
                className="rounded-lg border border-white/12 px-2 py-1 text-[9px] font-semibold uppercase tracking-wide hover:bg-white/10 disabled:opacity-40"
                style={{ color: 'var(--ink-dim)' }}
                disabled={cancelAllDisabled}
                title="Uses indexer placements: active orders for your wallet on this pair."
                onClick={() => void onCancelAllMyResting()}
              >
                Cancel all mine
              </button>
            )}
          </div>
        </div>
      </div>
      <div className="grid grid-cols-1 gap-2 flex-1 min-h-0">
        <BookSideColumn
          title="Asks"
          pairAddress={pairAddress}
          side="ask"
          priceLabel={priceLabel}
          sizeLabel={baseSymbol}
          sizeDecimals={pair?.asset_0.decimals ?? 6}
          walletAddress={walletAddress}
          isPairPaused={isPairPaused}
          isWalletConnected={isWalletConnected}
          openWalletModal={openWalletModal}
          cancelMutation={cancelLimitOrderMutation}
          onPrefillLimitTicket={onPrefillLimitTicket}
          displayInverted={displayInverted}
          limitPriceScale={limitPriceDecimalsFromPair(pair)}
        />
        <BookSideColumn
          title="Bids"
          pairAddress={pairAddress}
          side="bid"
          priceLabel={priceLabel}
          sizeLabel={quoteSymbol}
          sizeDecimals={pair?.asset_1.decimals ?? 6}
          walletAddress={walletAddress}
          isPairPaused={isPairPaused}
          isWalletConnected={isWalletConnected}
          openWalletModal={openWalletModal}
          cancelMutation={cancelLimitOrderMutation}
          onPrefillLimitTicket={onPrefillLimitTicket}
          displayInverted={displayInverted}
          limitPriceScale={limitPriceDecimalsFromPair(pair)}
        />
      </div>
    </div>
  )
}
