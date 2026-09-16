import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { renderHook, waitFor } from '@testing-library/react'
import type { ReactNode } from 'react'
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { useWalletStore } from '@/hooks/useWallet'

const TOKEN = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const WALLET = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    COMMUNITY_TAX_CODE_ID: 11630,
    isCommunityTaxEnabled: () => true,
  }
})

vi.mock('@/services/terraclassic/communityTaxToken', () => ({
  queryCommunityTaxConfig: vi.fn(),
  queryCommunityTaxIsExempt: vi.fn(),
  queryTaxPreview: vi.fn(),
}))

import { queryCommunityTaxConfig, queryCommunityTaxIsExempt } from '@/services/terraclassic/communityTaxToken'
import { useCommunityTaxSellBps } from '../useCommunityTaxSellBps'

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>
}

describe('useCommunityTaxSellBps (#1267 catalog pin ≠ sell detection)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    vi.mocked(queryCommunityTaxIsExempt).mockResolvedValue({
      address: WALLET,
      protocol: false,
      manager: false,
    })
  })

  it('AC4: GetConfig sell_bps arms extra-debit when live code_id is not the catalog pin', async () => {
    vi.mocked(queryCommunityTaxConfig).mockResolvedValue({
      sell_bps: 500,
      buy_bps: 0,
    } as never)
    const { result } = renderHook(() => useCommunityTaxSellBps(TOKEN), { wrapper })
    await waitFor(() => expect(result.current.isTaxToken).toBe(true))
    expect(result.current.sellBps).toBe(500)
    expect(result.current.extraDebitUnresolved).toBe(false)
    expect(queryCommunityTaxConfig).toHaveBeenCalled()
  })

  it('honest CW20 unknown-query is not tax', async () => {
    vi.mocked(queryCommunityTaxConfig).mockRejectedValue(new Error('unknown variant `get_config`'))
    const { result } = renderHook(() => useCommunityTaxSellBps(TOKEN), { wrapper })
    await waitFor(() => expect(result.current.isLoading).toBe(false))
    expect(result.current.isTaxToken).toBe(false)
    expect(result.current.sellBps).toBeNull()
    expect(result.current.extraDebitUnresolved).toBe(false)
  })

  it('unknown exempt keeps sell_bps (fail closed)', async () => {
    vi.mocked(queryCommunityTaxConfig).mockResolvedValue({ sell_bps: 500, buy_bps: 0 } as never)
    vi.mocked(queryCommunityTaxIsExempt).mockImplementation(() => new Promise(() => undefined) as never)
    const { result } = renderHook(() => useCommunityTaxSellBps(TOKEN), { wrapper })
    await waitFor(() => expect(result.current.isTaxToken).toBe(true))
    expect(result.current.sellBps).toBe(500)
  })
})
