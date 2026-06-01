import { describe, expect, it, vi, beforeEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'

import { useLimitLadderPlacementPlan } from '../useLimitLadderPlacementPlan'
import * as indexerClient from '@/services/indexer/client'

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof indexerClient>()
  return {
    ...actual,
    getPairLimitBookPage: vi.fn(),
    getPairLimitBookInsertHints: vi.fn(),
  }
})

function wrapper() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  )
}

const rungs = [
  { price: '0.95', amountRaw: '100' },
  { price: '1.05', amountRaw: '100' },
]

describe('useLimitLadderPlacementPlan', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getPairLimitBookPage).mockResolvedValue({
      side: 'bid',
      orders: [],
      has_more: false,
      next_after_order_id: null,
    })
    vi.mocked(indexerClient.getPairLimitBookInsertHints).mockResolvedValue({
      side: 'bid',
      budget_exhausted: false,
      hints: rungs.map((r) => ({
        price: r.price,
        predecessor_order_id: null,
        resolved: true,
        reason: 'head',
      })),
    })
  })

  it('fetches indexer depth + hints', async () => {
    const { result } = renderHook(
      () =>
        useLimitLadderPlacementPlan({
          pairAddress: 'terra1pair',
          side: 'bid',
          startPrice: '0.95',
          endPrice: '1.05',
          count: 2,
          rungs,
          maxAdjustSteps: 32,
        }),
      { wrapper: wrapper() }
    )
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(result.current.data?.path).toBe('thin_ladder')
    expect(indexerClient.getPairLimitBookInsertHints).toHaveBeenCalled()
    expect(indexerClient.getPairLimitBookPage).toHaveBeenCalled()
  })

  it('degrades when indexer fails', async () => {
    vi.mocked(indexerClient.getPairLimitBookPage).mockRejectedValue(new Error('502'))
    const { result } = renderHook(
      () =>
        useLimitLadderPlacementPlan({
          pairAddress: 'terra1pair',
          side: 'bid',
          startPrice: '0.95',
          endPrice: '1.05',
          count: 2,
          rungs,
          maxAdjustSteps: 32,
        }),
      { wrapper: wrapper() }
    )
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(result.current.data?.probeDegraded).toBe(true)
  })
})
