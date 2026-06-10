import { describe, expect, it, vi, beforeEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { prefetchTradePairWorkspace } from '@/utils/tradePairPrefetch'
import { useLimitBookInfinite } from '@/hooks/useLimitBookInfinite'
import * as indexerClient from '@/services/indexer/client'

const OTHER = 'terra1pair0000000000000000000000000000000002'

vi.mock('@/services/indexer/client', () => ({
  getPair: vi.fn(),
  getCandles: vi.fn(),
  getTrades: vi.fn(),
  getPairLimitBookPage: vi.fn(),
}))

function wrapper(client: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>
  }
}

describe('prefetchTradePairWorkspace infinite seed (GitLab #354)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(indexerClient.getPair).mockResolvedValue({} as never)
    vi.mocked(indexerClient.getCandles).mockResolvedValue([])
    vi.mocked(indexerClient.getTrades).mockResolvedValue([])
    vi.mocked(indexerClient.getPairLimitBookPage).mockResolvedValue({
      side: 'bid',
      orders: [{ order_id: 1, owner: 'terra1a', side: 'bid', price: '1', remaining: '100' }],
      has_more: false,
      next_after_order_id: null,
    })
  })

  it('useLimitBookInfinite succeeds when cache was seeded by prefetchTradePairWorkspace', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })

    prefetchTradePairWorkspace(queryClient, OTHER)

    await vi.waitFor(() => {
      expect(indexerClient.getPairLimitBookPage).toHaveBeenCalled()
    })

    const { result } = renderHook(() => useLimitBookInfinite(OTHER, 'bid'), {
      wrapper: wrapper(queryClient),
    })

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true)
      expect(result.current.data?.pages).toHaveLength(1)
      expect(result.current.data?.pages[0]?.orders).toHaveLength(1)
    })
    expect(result.current.isError).toBe(false)
  })
})
