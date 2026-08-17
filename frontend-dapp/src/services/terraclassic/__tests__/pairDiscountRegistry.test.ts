import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn(),
  queryContractRaw: vi.fn(),
}))

import { queryContract, queryContractRaw } from '@/services/terraclassic/queries'
import { getPairDiscountRegistry } from '../pairDiscountRegistry'
import { pairDiscountRegistryRawKeyB64 } from '@/utils/pairDiscountRegistry'

const mockedRaw = vi.mocked(queryContractRaw)
const mockedSmart = vi.mocked(queryContract)

const PAIR = 'terra1pair00000000000000000000000000000001'
const REGISTRY = 'terra1wcczsdk7jwj99n3my6wx8wr4ee0hn6yaapgd792lgx5elrdtrn2scfnecz'

describe('getPairDiscountRegistry (GitLab #538 smart-query-first / #537 raw fallback)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns null from GetDiscountRegistry without hitting raw state', async () => {
    mockedSmart.mockResolvedValueOnce({ registry: null })
    await expect(getPairDiscountRegistry(PAIR)).resolves.toBeNull()
    expect(mockedSmart).toHaveBeenCalledWith(PAIR, { get_discount_registry: {} })
    expect(mockedRaw).not.toHaveBeenCalled()
  })

  it('returns the stored registry from GetDiscountRegistry', async () => {
    mockedSmart.mockResolvedValueOnce({ registry: REGISTRY })
    await expect(getPairDiscountRegistry(PAIR)).resolves.toBe(REGISTRY)
    expect(mockedRaw).not.toHaveBeenCalled()
  })

  it('accepts discount_registry field on the smart-query payload', async () => {
    mockedSmart.mockResolvedValueOnce({ discount_registry: REGISTRY })
    await expect(getPairDiscountRegistry(PAIR)).resolves.toBe(REGISTRY)
    expect(mockedRaw).not.toHaveBeenCalled()
  })

  it('falls back to LCD raw when GetDiscountRegistry is missing (1.13.x wasm)', async () => {
    mockedSmart.mockRejectedValueOnce(new Error('unknown variant `get_discount_registry`'))
    mockedRaw.mockResolvedValueOnce(btoa(JSON.stringify(REGISTRY)))
    await expect(getPairDiscountRegistry(PAIR)).resolves.toBe(REGISTRY)
    expect(mockedRaw).toHaveBeenCalledWith(PAIR, pairDiscountRegistryRawKeyB64())
  })

  it('raw fallback returns null for an unwired pair', async () => {
    mockedSmart.mockRejectedValueOnce(new Error('unknown variant'))
    mockedRaw.mockResolvedValueOnce(btoa('null'))
    await expect(getPairDiscountRegistry(PAIR)).resolves.toBeNull()
  })

  it('rethrows the smart-query error when raw fallback also fails', async () => {
    mockedSmart.mockRejectedValueOnce(new Error('unknown variant `get_discount_registry`'))
    mockedRaw.mockRejectedValueOnce(new Error('Raw query failed: 403'))
    await expect(getPairDiscountRegistry(PAIR)).rejects.toThrow('unknown variant `get_discount_registry`')
  })
})
