import { useInfiniteQuery } from '@tanstack/react-query'
import { getPairLimitBookPage } from '@/services/indexer/client'
import { LIMIT_BOOK_UI_PAGE_SIZE, limitBookPageQueryKey } from '@/utils/limitBookPagination'

/**
 * Paginated on-chain limit book for one side (`GET .../limit-book` via indexer → LCD).
 * Additional depth loads on demand via `fetchNextPage` — does not block the main thread.
 */
export function useLimitBookInfinite(pairAddress: string, side: 'bid' | 'ask') {
  return useInfiniteQuery({
    queryKey: limitBookPageQueryKey(pairAddress, side),
    queryFn: ({ pageParam }) =>
      getPairLimitBookPage(pairAddress, side, {
        limit: LIMIT_BOOK_UI_PAGE_SIZE,
        afterOrderId: pageParam,
      }),
    initialPageParam: undefined as number | undefined,
    getNextPageParam: (last) =>
      last.has_more && last.next_after_order_id != null ? last.next_after_order_id : undefined,
    enabled: pairAddress.startsWith('terra1'),
    staleTime: 10_000,
  })
}
