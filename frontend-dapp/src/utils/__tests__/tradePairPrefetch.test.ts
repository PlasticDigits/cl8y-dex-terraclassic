import { describe, it, expect, vi, beforeEach } from 'vitest'
import { QueryClient } from '@tanstack/react-query'
import { prefetchTradePairWorkspace } from '@/utils/tradePairPrefetch'
import * as indexerClient from '@/services/indexer/client'

const PAIR = 'terra1pair0000000000000000000000000000000001'

vi.mock('@/services/indexer/client', () => ({
  getPair: vi.fn(),
  getCandles: vi.fn(),
  getTrades: vi.fn(),
  getPairLimitBookPage: vi.fn(),
}))

describe('prefetchTradePairWorkspace', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(indexerClient.getPair).mockResolvedValue({} as never)
    vi.mocked(indexerClient.getCandles).mockResolvedValue([])
    vi.mocked(indexerClient.getTrades).mockResolvedValue([])
    vi.mocked(indexerClient.getPairLimitBookPage).mockResolvedValue({
      side: 'bid',
      orders: [],
      has_more: false,
      next_after_order_id: null,
    })
  })

  it('prefetches pair metadata, candles, tape, and both book sides (GitLab #180)', async () => {
    const queryClient = new QueryClient()
    prefetchTradePairWorkspace(queryClient, PAIR)

    await vi.waitFor(() => {
      expect(indexerClient.getPair).toHaveBeenCalledWith(PAIR)
      expect(indexerClient.getCandles).toHaveBeenCalledWith(PAIR, '1h')
      expect(indexerClient.getTrades).toHaveBeenCalledWith(PAIR, 80)
      expect(indexerClient.getPairLimitBookPage).toHaveBeenCalledWith(PAIR, 'bid', expect.any(Object))
      expect(indexerClient.getPairLimitBookPage).toHaveBeenCalledWith(PAIR, 'ask', expect.any(Object))
    })
  })

  it('seeds limit-book cache as InfiniteData for useLimitBookInfinite (GitLab #354)', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    prefetchTradePairWorkspace(queryClient, PAIR)

    await vi.waitFor(() => {
      const bidData = queryClient.getQueryData(['limitBookPage', PAIR, 'bid'])
      expect(bidData).toBeDefined()
      expect(bidData).toHaveProperty('pages')
      expect(bidData).toHaveProperty('pageParams')
      expect((bidData as { pages: unknown[] }).pages).toHaveLength(1)
    })
  })

  it('ignores invalid pair addresses', async () => {
    const queryClient = new QueryClient()
    prefetchTradePairWorkspace(queryClient, 'not-a-pair')
    await new Promise((r) => setTimeout(r, 20))
    expect(indexerClient.getPair).not.toHaveBeenCalled()
    expect(indexerClient.getCandles).not.toHaveBeenCalled()
  })
})
