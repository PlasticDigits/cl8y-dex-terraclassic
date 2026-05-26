import { describe, expect, it, vi, beforeEach } from 'vitest'
import { renderHook, waitFor, act } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { getPairLimitBookPage } from '@/services/indexer/client'
import { useLimitBookInfinite } from '../useLimitBookInfinite'
import { LIMIT_BOOK_UI_PAGE_SIZE } from '@/utils/limitBookPagination'

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return { ...actual, getPairLimitBookPage: vi.fn() }
})

const PAIR = 'terra1pair00000000000000000000000000000001'

function wrapper(client: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>
  }
}

describe('useLimitBookInfinite', () => {
  beforeEach(() => {
    vi.mocked(getPairLimitBookPage).mockReset()
  })

  it('requests first page with UI page size and follows keyset cursor', async () => {
    vi.mocked(getPairLimitBookPage).mockImplementation(async (_pair, _side, params) => {
      if (params?.afterOrderId != null) {
        return {
          side: 'bid',
          orders: [{ order_id: 2, owner: 'terra1b', side: 'bid', price: '0.9', remaining: '50' }],
          has_more: false,
          next_after_order_id: null,
        }
      }
      return {
        side: 'bid',
        orders: [{ order_id: 1, owner: 'terra1a', side: 'bid', price: '1', remaining: '100' }],
        has_more: true,
        next_after_order_id: 1,
      }
    })

    const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    const { result } = renderHook(() => useLimitBookInfinite(PAIR, 'bid'), {
      wrapper: wrapper(client),
    })

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true)
      expect(result.current.hasNextPage).toBe(true)
    })
    expect(getPairLimitBookPage).toHaveBeenCalledWith(PAIR, 'bid', {
      limit: LIMIT_BOOK_UI_PAGE_SIZE,
      afterOrderId: undefined,
    })

    await act(async () => {
      const res = await result.current.fetchNextPage()
      expect(res.isError).toBe(false)
    })
    await waitFor(() => expect(result.current.data?.pages).toHaveLength(2))
    expect(getPairLimitBookPage).toHaveBeenLastCalledWith(PAIR, 'bid', {
      limit: LIMIT_BOOK_UI_PAGE_SIZE,
      afterOrderId: 1,
    })
  })
})
