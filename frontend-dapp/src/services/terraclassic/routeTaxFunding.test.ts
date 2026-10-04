import { beforeEach, describe, expect, it, vi } from 'vitest'
import { queryContract } from './queries'
import { assertRouteTaxFunding, previewRouteTax, ROUTE_TAX_FUNDING_UNAVAILABLE } from './routeTaxFunding'
import type { SwapOperation } from './router'

vi.mock('./queries', () => ({ queryContract: vi.fn() }))
const operations: SwapOperation[] = [
  {
    terra_swap: {
      offer_asset_info: { token: { contract_addr: 'terra1offer' } },
      ask_asset_info: { token: { contract_addr: 'terra1ask' } },
    },
  },
]

describe('route funding discovery (#1381)', () => {
  beforeEach(() => vi.resetAllMocks())
  it('uses live upgraded router capability independently of catalog configuration', async () => {
    vi.mocked(queryContract).mockResolvedValue({ funding_version: 1 })
    expect(await assertRouteTaxFunding(operations)).toBe(1)
    expect(queryContract).toHaveBeenCalledTimes(1)
  })
  it('rejects a positive tax token on a legacy router before broadcasting', async () => {
    vi.mocked(queryContract).mockResolvedValueOnce({}).mockResolvedValueOnce({ sell_bps: 100, buy_bps: 450 })
    await expect(assertRouteTaxFunding(operations)).rejects.toThrow(ROUTE_TAX_FUNDING_UNAVAILABLE)
  })
  it('rejects a legacy Send-tax API despite unsupported GetConfig', async () => {
    vi.mocked(queryContract)
      .mockResolvedValueOnce({})
      .mockRejectedValueOnce(new Error('unknown variant get_config'))
      .mockResolvedValueOnce({ tax_rate: '0.05' })
    await expect(assertRouteTaxFunding(operations)).rejects.toThrow(ROUTE_TAX_FUNDING_UNAVAILABLE)
  })
  it('allows verified ordinary legacy assets only after all known tax APIs reject', async () => {
    vi.mocked(queryContract).mockResolvedValueOnce({}).mockRejectedValue(new Error('unknown variant'))
    expect(await assertRouteTaxFunding(operations)).toBeUndefined()
    expect(queryContract).toHaveBeenCalledTimes(7)
  })
  it.each(['503 unavailable', 'timeout', '429 rate limited'])('blocks capability failure: %s', async (message) => {
    vi.mocked(queryContract).mockRejectedValue(new Error(message))
    await expect(assertRouteTaxFunding(operations)).rejects.toThrow(ROUTE_TAX_FUNDING_UNAVAILABLE)
  })
  it('rejects malformed, excess-budget and old-version previews', async () => {
    for (const preview of [
      { funding_version: 0, gross: '1010000', economic: '1000000', net: '1000000' },
      { funding_version: 1, gross: '1010000', economic: '1020000', net: '1000000' },
      { funding_version: 1, gross: '1010000', economic: '1'.repeat(100000), net: '1000000' },
    ]) {
      vi.mocked(queryContract).mockResolvedValue(preview)
      await expect(
        previewRouteTax({
          token: 'terra1alpha',
          kind: 'sell',
          amount: '1010000',
          trader: 'terra1synthetic',
          pair: 'terra1pair',
        })
      ).rejects.toThrow(ROUTE_TAX_FUNDING_UNAVAILABLE)
    }
  })
})
