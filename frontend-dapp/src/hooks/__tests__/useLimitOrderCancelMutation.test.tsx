import { describe, expect, it, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { cancelLimitOrder, cancelLimitOrders } from '@/services/terraclassic/pair'
import { useLimitOrderCancelMutation } from '@/hooks/useLimitOrderCancelMutation'
import { limitOrderStatusQueryKey, recentlyCancelledOrderIdsQueryKey } from '@/hooks/useLimitOrderStatuses'

vi.mock('@/services/terraclassic/pair', () => ({
  cancelLimitOrder: vi.fn(),
  cancelLimitOrders: vi.fn(),
}))

vi.mock('@/lib/sounds', () => ({
  sounds: { playSuccess: vi.fn(), playError: vi.fn() },
}))

const PAIR = 'terra1pair00000000000000000000000000000001'
const WALLET = 'terra1wallet00000000000000000000000000001'

function wrapper(client: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>
  }
}

describe('useLimitOrderCancelMutation (GitLab #530)', () => {
  beforeEach(() => {
    vi.mocked(cancelLimitOrder).mockReset()
    vi.mocked(cancelLimitOrders).mockReset()
    vi.mocked(cancelLimitOrder).mockResolvedValue('TX_CANCEL')
    vi.mocked(cancelLimitOrders).mockResolvedValue('TX_BATCH')
  })

  it('broadcasts single cancel_limit_order', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const { result } = renderHook(() => useLimitOrderCancelMutation(PAIR, WALLET), {
      wrapper: wrapper(client),
    })
    await act(async () => {
      await result.current.mutateAsync(7)
    })
    expect(cancelLimitOrder).toHaveBeenCalledWith(WALLET, PAIR, 7)
    expect(client.getQueryData<number[]>(recentlyCancelledOrderIdsQueryKey(PAIR))).toEqual([7])
  })

  it('broadcasts batch cancel_limit_orders', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const { result } = renderHook(() => useLimitOrderCancelMutation(PAIR, WALLET), {
      wrapper: wrapper(client),
    })
    await act(async () => {
      await result.current.mutateAsync([2, 3])
    })
    expect(cancelLimitOrders).toHaveBeenCalledWith(WALLET, PAIR, [2, 3])
  })

  it('throws on indexed cancellation without broadcasting (A3)', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    client.setQueryData(['limitCancellations', PAIR], [{ order_id: 4 }])
    const { result } = renderHook(() => useLimitOrderCancelMutation(PAIR, WALLET), {
      wrapper: wrapper(client),
    })
    await expect(result.current.mutateAsync(4)).rejects.toThrow(/already been cancelled/)
    expect(cancelLimitOrder).not.toHaveBeenCalled()
  })

  it('throws on LCD Unknown without broadcasting (A2)', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    client.setQueryData(limitOrderStatusQueryKey(PAIR, 1), 'unknown')
    const { result } = renderHook(() => useLimitOrderCancelMutation(PAIR, WALLET), {
      wrapper: wrapper(client),
    })
    await expect(result.current.mutateAsync(1)).rejects.toThrow(/no longer on the book/)
    expect(cancelLimitOrder).not.toHaveBeenCalled()
  })

  it('throws on LCD ParkedRefund — Claim, not Cancel (AC4)', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    client.setQueryData(limitOrderStatusQueryKey(PAIR, 9), 'parked_refund')
    const { result } = renderHook(() => useLimitOrderCancelMutation(PAIR, WALLET), {
      wrapper: wrapper(client),
    })
    await expect(result.current.mutateAsync(9)).rejects.toThrow(/Claim refund/)
    expect(cancelLimitOrder).not.toHaveBeenCalled()
  })
})
