import { describe, expect, it, vi, beforeEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { useLimitOrderMakerFeeRates } from '@/hooks/useLimitOrderMakerFeeRates'
import { getPairFeeConfig } from '@/services/terraclassic/settings'
import { getTraderDiscount } from '@/services/terraclassic/feeDiscount'
import { getPairDiscountRegistry } from '@/services/terraclassic/pairDiscountRegistry'

const PAIR = 'terra1pair00000000000000000000000000000001'
const WALLET = 'terra1wallet000000000000000000000000000001'
const { REGISTRY } = vi.hoisted(() => ({
  REGISTRY: 'terra1feediscount000000000000000000000000001',
}))

vi.mock('@/services/terraclassic/settings', () => ({
  getPairFeeConfig: vi.fn(),
}))

vi.mock('@/services/terraclassic/feeDiscount', () => ({
  getTraderDiscount: vi.fn(),
}))

vi.mock('@/services/terraclassic/pairDiscountRegistry', () => ({
  getPairDiscountRegistry: vi.fn(),
}))

vi.mock('@/utils/constants', async (importActual) => {
  const actual = await importActual<typeof import('@/utils/constants')>()
  return {
    ...actual,
    FEE_DISCOUNT_CONTRACT_ADDRESS: REGISTRY,
  }
})

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  })
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>
}

describe('useLimitOrderMakerFeeRates pair registry gate (GitLab #537)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(getPairFeeConfig).mockResolvedValue({ fee_bps: 180, treasury: 'terra1treasury' })
    vi.mocked(getTraderDiscount).mockResolvedValue({
      discount_bps: 9500,
      limit_discount_bps: 10000,
      needs_deregister: false,
      registration_epoch: 1,
    })
  })

  it('uses full maker place fee when the pair has no discount_registry', async () => {
    vi.mocked(getPairDiscountRegistry).mockResolvedValue(null)
    const { result } = renderHook(() => useLimitOrderMakerFeeRates(PAIR, WALLET), { wrapper })
    await waitFor(() => expect(result.current.feeLoading).toBe(false))
    expect(result.current.pairDiscountApplies).toBe(false)
    expect(result.current.effectiveFeeBps).toBe(180)
    expect(result.current.makerPlacementFeeBps).toBe(90)
    expect(getTraderDiscount).not.toHaveBeenCalled()
  })

  it('applies limit_discount_bps when the pair registry matches VITE_FEE_DISCOUNT_ADDRESS', async () => {
    vi.mocked(getPairDiscountRegistry).mockResolvedValue(REGISTRY)
    const { result } = renderHook(() => useLimitOrderMakerFeeRates(PAIR, WALLET), { wrapper })
    await waitFor(() => expect(result.current.makerPlacementFeeBps).toBe(0))
    expect(result.current.pairDiscountApplies).toBe(true)
    expect(result.current.effectiveFeeBps).toBe(0)
    expect(getTraderDiscount).toHaveBeenCalled()
  })

  it('does not apply VITE discount when the pair registry is a different contract', async () => {
    vi.mocked(getPairDiscountRegistry).mockResolvedValue('terra1otherdiscountregistry0000000000000000000001')
    const { result } = renderHook(() => useLimitOrderMakerFeeRates(PAIR, WALLET), { wrapper })
    await waitFor(() => expect(result.current.feeLoading).toBe(false))
    expect(result.current.effectiveFeeBps).toBe(180)
    expect(result.current.makerPlacementFeeBps).toBe(90)
  })
})
