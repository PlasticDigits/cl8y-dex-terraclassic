import { useInfiniteQuery } from '@tanstack/react-query'
import { getPairLimitBookPage } from '@/services/indexer/client'
import { Spinner } from '@/components/ui'
import type { IndexerPair, IndexerShallowLimitOrder } from '@/types'
import { formatNum, formatTokenAmount } from '@/utils/formatAmount'

const PAGE_LIMIT = 45

function rawTotal(orders: IndexerShallowLimitOrder[]): bigint {
  return orders.reduce((acc, order) => {
    try {
      return acc + BigInt(order.remaining)
    } catch {
      return acc
    }
  }, 0n)
}

function formatBookPrice(raw: string): string {
  return formatNum(raw, 7)
    .replace(/(\.\d*?[1-9])0+$/, '$1')
    .replace(/\.0+$/, '')
}

function BookRow({
  order,
  side,
  sizeDecimals,
  maxRaw,
  cumulative,
}: {
  order: IndexerShallowLimitOrder
  side: 'bid' | 'ask'
  sizeDecimals: number
  maxRaw: bigint
  cumulative: string
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

  return (
    <li
      className="relative grid grid-cols-[1fr_1fr_1fr] items-center gap-2 rounded-lg px-2 py-1.5 text-[11px] font-mono tabular-nums overflow-hidden"
      title={`Order #${order.order_id} · raw remaining ${order.remaining}`}
    >
      <span
        className="absolute inset-y-0 right-0 pointer-events-none"
        style={{ width: `${Math.min(depthPct, 100)}%`, background: sideBg }}
      />
      <span className="relative font-semibold" style={{ color: sideColor }}>
        {formatBookPrice(order.price)}
      </span>
      <span className="relative text-right" style={{ color: 'var(--ink)' }}>
        {formatTokenAmount(order.remaining, sizeDecimals, 4)}
      </span>
      <span className="relative text-right" style={{ color: 'var(--ink-dim)' }}>
        {formatTokenAmount(cumulative, sizeDecimals, 4)}
      </span>
    </li>
  )
}

function BookSideColumn({
  title,
  pairAddress,
  side,
  priceLabel,
  sizeLabel,
  sizeDecimals,
}: {
  title: string
  pairAddress: string
  side: 'bid' | 'ask'
  priceLabel: string
  sizeLabel: string
  sizeDecimals: number
}) {
  const q = useInfiniteQuery({
    queryKey: ['limitBookPage', pairAddress, side],
    queryFn: ({ pageParam }) =>
      getPairLimitBookPage(pairAddress, side, {
        limit: PAGE_LIMIT,
        afterOrderId: pageParam,
      }),
    initialPageParam: undefined as number | undefined,
    getNextPageParam: (last) =>
      last.has_more && last.next_after_order_id != null ? last.next_after_order_id : undefined,
    enabled: pairAddress.startsWith('terra1'),
    staleTime: 10_000,
  })

  const orders = q.data?.pages.flatMap((p) => p.orders) ?? []
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

  return (
    <div className="flex flex-col min-h-0 h-full card-neo !p-3 overflow-hidden">
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
        <p className="text-xs" style={{ color: 'var(--ink-dim)' }}>
          Book unavailable (indexer or LCD).
        </p>
      )}
      {!q.isLoading && !q.isError && (
        <>
          <div
            className="grid grid-cols-[1fr_1fr_1fr] gap-2 px-2 pb-1 text-[9px] font-semibold uppercase tracking-wide"
            style={{ color: 'var(--ink-subtle)' }}
          >
            <span>Price {priceLabel}</span>
            <span className="text-right">Size {sizeLabel}</span>
            <span className="text-right">Total</span>
          </div>
          <ul className="space-y-1 flex-1 overflow-y-auto min-h-[96px] pr-1">
            {orders.length === 0 && (
              <li
                className="flex h-full min-h-[96px] items-center justify-center rounded-xl border border-dashed px-3 text-center text-[11px]"
                style={{ borderColor: 'var(--line)', color: 'var(--ink-subtle)' }}
              >
                {emptyLabel}
              </li>
            )}
            {rows.map(({ order, cumulative }) => (
              <BookRow
                key={`${side}-${order.order_id}`}
                order={order}
                side={side}
                sizeDecimals={sizeDecimals}
                maxRaw={maxRaw}
                cumulative={cumulative}
              />
            ))}
          </ul>
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

export function OrderBookPanel({ pairAddress, pair }: { pairAddress: string; pair?: IndexerPair }) {
  if (!pairAddress.startsWith('terra1')) {
    return (
      <div className="card-neo !p-4 text-sm" style={{ color: 'var(--ink-dim)' }}>
        Select a pair to view the order book.
      </div>
    )
  }

  const baseSymbol = pair?.asset_0.symbol ?? 'token0'
  const quoteSymbol = pair?.asset_1.symbol ?? 'token1'
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
              title="Shows open resting limit orders only. Swaps against AMM pool liquidity appear in Recent trades, not in this book."
            >
              Resting limit orders only — AMM pool depth is not shown here.
            </p>
          </div>
          <div
            className="rounded-full border px-2 py-0.5 text-[9px] font-semibold uppercase tracking-wide"
            style={{ borderColor: 'var(--line)', color: 'var(--ink-subtle)' }}
          >
            {priceLabel}
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
        />
        <BookSideColumn
          title="Bids"
          pairAddress={pairAddress}
          side="bid"
          priceLabel={priceLabel}
          sizeLabel={quoteSymbol}
          sizeDecimals={pair?.asset_1.decimals ?? 6}
        />
      </div>
    </div>
  )
}
