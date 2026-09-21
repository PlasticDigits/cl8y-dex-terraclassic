import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { CW20_TOKEN_INFO_CACHE_KEY, resetCw20TokenInfoInFlightForTests } from '@/utils/tokenDisplay'
import { USTR_CW20_ADDRESS } from '@/utils/tokenRegistry'

vi.mock('@/services/indexer/client', () => ({
  getTokens: vi.fn().mockResolvedValue([]),
}))

vi.mock('@/utils/tokenDisplay', async (importActual) => {
  const actual = await importActual<typeof import('@/utils/tokenDisplay')>()
  return {
    ...actual,
    fetchCW20TokenInfo: vi.fn(),
  }
})

import { getTokens } from '@/services/indexer/client'
import { fetchCW20TokenInfo } from '@/utils/tokenDisplay'
import { useAssetDecimals } from '@/hooks/useAssetDecimals'

const UNKNOWN = 'terra1from00000000000000000000000000000001'

function wrapper(client: QueryClient) {
  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  )
}

describe('useAssetDecimals (#1255)', () => {
  beforeEach(() => {
    localStorage.removeItem(CW20_TOKEN_INFO_CACHE_KEY)
    resetCw20TokenInfoInFlightForTests()
    vi.mocked(getTokens).mockReset()
    vi.mocked(getTokens).mockResolvedValue([])
    vi.mocked(fetchCW20TokenInfo).mockReset()
  })

  afterEach(() => {
    localStorage.removeItem(CW20_TOKEN_INFO_CACHE_KEY)
  })

  it('resolves listed USTR from registry without LCD', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    const { result } = renderHook(() => useAssetDecimals(USTR_CW20_ADDRESS, { lcdEnabled: true }), {
      wrapper: wrapper(client),
    })
    await waitFor(() => expect(result.current.decimals).toBe(18))
    expect(result.current.source).toBe('registry')
    expect(fetchCW20TokenInfo).not.toHaveBeenCalled()
  })

  it('resolves uluna as 6 from registry', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    const { result } = renderHook(() => useAssetDecimals('uluna', { lcdEnabled: false }), {
      wrapper: wrapper(client),
    })
    expect(result.current.decimals).toBe(6)
    expect(result.current.source).toBe('registry')
  })

  it('uses LCD 18 for unknown factory CW20', async () => {
    vi.mocked(fetchCW20TokenInfo).mockResolvedValue({
      name: 'Gem',
      symbol: 'GEMX',
      decimals: 18,
      decimalsHostile: false,
      total_supply: '0',
    })
    const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    const { result } = renderHook(() => useAssetDecimals(UNKNOWN, { lcdEnabled: true }), {
      wrapper: wrapper(client),
    })
    await waitFor(() => expect(result.current.decimals).toBe(18))
    expect(result.current.source).toBe('lcd')
  })

  it('stays unresolved (not 6) while LCD hangs and indexer is empty', async () => {
    vi.mocked(fetchCW20TokenInfo).mockImplementation(() => new Promise(() => {}))
    const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    const { result } = renderHook(() => useAssetDecimals(UNKNOWN, { lcdEnabled: true }), {
      wrapper: wrapper(client),
    })
    await waitFor(() => expect(result.current.pending).toBe(true))
    expect(result.current.decimals).toBeNull()
  })

  it('fails closed on hostile LCD decimals', async () => {
    vi.mocked(fetchCW20TokenInfo).mockResolvedValue({
      name: 'Bad',
      symbol: 'BAD',
      decimals: null,
      decimalsHostile: true,
      total_supply: '0',
    })
    const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    const { result } = renderHook(() => useAssetDecimals(UNKNOWN, { lcdEnabled: true }), {
      wrapper: wrapper(client),
    })
    await waitFor(() => expect(result.current.pending).toBe(false))
    expect(result.current.decimals).toBeNull()
  })

  it('uses indexer 18 while LCD is in flight', async () => {
    vi.mocked(fetchCW20TokenInfo).mockImplementation(() => new Promise(() => {}))
    vi.mocked(getTokens).mockResolvedValue([
      {
        id: 1,
        contract_address: UNKNOWN,
        denom: null,
        is_cw20: true,
        name: 'Gem',
        symbol: 'GEMX',
        decimals: 18,
        logo_url: null,
        coingecko_id: null,
        cmc_id: null,
      },
    ])
    const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    const { result } = renderHook(() => useAssetDecimals(UNKNOWN, { lcdEnabled: true }), {
      wrapper: wrapper(client),
    })
    await waitFor(() => expect(result.current.decimals).toBe(18))
    expect(result.current.source).toBe('indexer')
  })
})
