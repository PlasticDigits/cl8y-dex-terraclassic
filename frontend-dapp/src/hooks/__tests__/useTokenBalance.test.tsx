import { describe, expect, it, vi, beforeEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { useLimitOrderEscrowBalance } from '@/hooks/useLimitOrderEscrowBalance'
import { useTokenBalance } from '@/hooks/useTokenBalance'
import { getTokenBalance } from '@/services/terraclassic/queries'

vi.mock('@/services/terraclassic/queries', () => ({
  getTokenBalance: vi.fn(),
}))

const WALLET = 'terra1wallet00000000000000000000000000001'
const CW20 = 'terra1cccccccccccccccccccccccccccccccccccc'

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>
}

describe('useTokenBalance (GitLab #231)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(getTokenBalance).mockResolvedValue('123456')
  })

  it('re-exports useLimitOrderEscrowBalance so ladder gates and limit place share one hook', () => {
    expect(useTokenBalance).toBe(useLimitOrderEscrowBalance)
  })

  it('fetches native uusd via bank by_denom (wrap USTC pay)', async () => {
    const { result } = renderHook(() => useTokenBalance(WALLET, 'uusd'), { wrapper })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(result.current.data).toBe('123456')
    expect(getTokenBalance).toHaveBeenCalledWith(WALLET, { native_token: { denom: 'uusd' } })
  })

  it('fetches native uluna via shared tokenBalance key', async () => {
    const { result } = renderHook(() => useTokenBalance(WALLET, 'uluna'), { wrapper })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(getTokenBalance).toHaveBeenCalledWith(WALLET, { native_token: { denom: 'uluna' } })
  })

  it('fetches CW20 balances', async () => {
    const { result } = renderHook(() => useTokenBalance(WALLET, CW20), { wrapper })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(getTokenBalance).toHaveBeenCalledWith(WALLET, { token: { contract_addr: CW20 } })
  })

  it('stays disabled without wallet or token id', () => {
    const { result: noWallet } = renderHook(() => useTokenBalance(null, 'uusd'), { wrapper })
    expect(noWallet.current.fetchStatus).toBe('idle')
    expect(getTokenBalance).not.toHaveBeenCalled()

    const { result: noToken } = renderHook(() => useTokenBalance(WALLET, ''), { wrapper })
    expect(noToken.current.fetchStatus).toBe('idle')
    expect(getTokenBalance).not.toHaveBeenCalled()
  })
})
