import { useInfiniteQuery } from '@tanstack/react-query'
import { getPairLimitBookPage } from '@/services/indexer/client'
import { Spinner } from '@/components/ui'

const PAGE_LIMIT = 45

function BookSideColumn({ title, pairAddress, side }: { title: string; pairAddress: string; side: 'bid' | 'ask' }) {
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

  return (
    <div className="flex flex-col min-h-0 h-full card-neo !p-3">
      <div className="text-xs font-semibold uppercase tracking-wide mb-2 shrink-0" style={{ color: 'var(--ink-dim)' }}>
        {title}
      </div>
      {q.isLoading && (
        <div className="flex justify-center py-6">
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
          <ul className="text-[11px] font-mono space-y-0.5 flex-1 overflow-y-auto min-h-[120px] pr-1">
            {orders.length === 0 && <li className="opacity-70">(empty)</li>}
            {orders.map((o) => (
              <li key={`${side}-${o.order_id}`} className="tabular-nums">
                #{o.order_id} · {o.price} · {o.remaining}
              </li>
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

export function OrderBookPanel({ pairAddress }: { pairAddress: string }) {
  if (!pairAddress.startsWith('terra1')) {
    return (
      <div className="card-neo !p-4 text-sm" style={{ color: 'var(--ink-dim)' }}>
        Select a pair to view the order book.
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-2 h-full min-h-0">
      <h2 className="text-xs font-semibold uppercase tracking-wide shrink-0" style={{ color: 'var(--ink)' }}>
        Order book
      </h2>
      <div className="grid grid-cols-2 gap-2 flex-1 min-h-0">
        <BookSideColumn title="Bids" pairAddress={pairAddress} side="bid" />
        <BookSideColumn title="Asks" pairAddress={pairAddress} side="ask" />
      </div>
    </div>
  )
}
