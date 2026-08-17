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

describe('getPairDiscountRegistry (GitLab #537)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns null when raw state is JSON null (unwired pair)', async () => {
    mockedRaw.mockResolvedValueOnce(btoa('null'))
    await expect(getPairDiscountRegistry(PAIR)).resolves.toBeNull()
    expect(mockedRaw).toHaveBeenCalledWith(PAIR, pairDiscountRegistryRawKeyB64())
    expect(mockedSmart).not.toHaveBeenCalled()
  })

  it('returns the stored registry address from raw state', async () => {
    mockedRaw.mockResolvedValueOnce(btoa(JSON.stringify(REGISTRY)))
    await expect(getPairDiscountRegistry(PAIR)).resolves.toBe(REGISTRY)
  })

  it('falls back to get_discount_registry when raw LCD fails', async () => {
    mockedRaw.mockRejectedValueOnce(new Error('Raw query failed: 403'))
    mockedSmart.mockResolvedValueOnce({ discount_registry: REGISTRY })
    await expect(getPairDiscountRegistry(PAIR)).resolves.toBe(REGISTRY)
    expect(mockedSmart).toHaveBeenCalledWith(PAIR, { get_discount_registry: {} })
  })

  it('rethrows the raw error when smart fallback also fails', async () => {
    mockedRaw.mockRejectedValueOnce(new Error('Raw query failed: 403'))
    mockedSmart.mockRejectedValueOnce(new Error('unknown variant'))
    await expect(getPairDiscountRegistry(PAIR)).rejects.toThrow('Raw query failed: 403')
  })
})
