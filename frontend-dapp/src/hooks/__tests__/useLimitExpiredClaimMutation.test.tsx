import { describe, expect, it, vi, beforeEach } from 'vitest'
import { renderHook, waitFor, act } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { claimExpiredLimitOrder, claimExpiredLimitOrders } from '@/services/terraclassic/pair'
import { useLimitExpiredClaimMutation } from '@/hooks/useLimitExpiredClaimMutation'

vi.mock('@/services/terraclassic/pair', () => ({
  claimExpiredLimitOrder: vi.fn(),
  claimExpiredLimitOrders: vi.fn(),
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

describe('useLimitExpiredClaimMutation (GitLab #253)', () => {
  beforeEach(() => {
    vi.mocked(claimExpiredLimitOrder).mockReset()
    vi.mocked(claimExpiredLimitOrders).mockReset()
    vi.mocked(claimExpiredLimitOrder).mockResolvedValue('TX_SINGLE')
    vi.mocked(claimExpiredLimitOrders).mockResolvedValue('TX_BATCH')
  })

  it('uses single claim execute msg for one id', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const { result } = renderHook(() => useLimitExpiredClaimMutation(PAIR, WALLET), {
      wrapper: wrapper(client),
    })

    await act(async () => {
      await result.current.mutateAsync(9)
    })

    expect(claimExpiredLimitOrder).toHaveBeenCalledWith(WALLET, PAIR, 9)
    expect(claimExpiredLimitOrders).not.toHaveBeenCalled()
  })

  it('uses batch claim execute msg for multiple ids', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const { result } = renderHook(() => useLimitExpiredClaimMutation(PAIR, WALLET), {
      wrapper: wrapper(client),
    })

    await act(async () => {
      await result.current.mutateAsync([2, 2, 3])
    })

    expect(claimExpiredLimitOrders).toHaveBeenCalledWith(WALLET, PAIR, [2, 3])
    expect(claimExpiredLimitOrder).not.toHaveBeenCalled()
  })

  it('rejects missing wallet', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    const { result } = renderHook(() => useLimitExpiredClaimMutation(PAIR, undefined), {
      wrapper: wrapper(client),
    })

    await expect(result.current.mutateAsync(1)).rejects.toThrow(/Connect wallet/)
    await waitFor(() => expect(result.current.isError).toBe(true))
  })
})
