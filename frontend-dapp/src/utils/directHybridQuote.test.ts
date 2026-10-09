import { queryContract } from '@/services/terraclassic/queries'
import { describe, expect, it, vi } from 'vitest'
import { quoteDirectHybridSwap } from './directHybridQuote'
vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn(() => {
    throw new Error('RPC forbidden')
  }),
}))
describe('manual book unsupported local metadata #1452', () => {
  it('blocks explicitly instead of repricing a pool-only route or querying the chain', async () => {
    await expect(
      quoteDirectHybridSwap({
        pairAddress: 'pair',
        fromToken: 'terra1from',
        toToken: 'terra1to',
        offerAssetInfo: { token: { contract_addr: 'terra1from' } },
        askAssetInfo: { token: { contract_addr: 'terra1to' } },
        simRaw: '1000',
        hybrid: { pool_input: '900', book_input: '100', max_maker_fills: 8 },
        maxSpreadStr: '0.05',
      })
    ).rejects.toThrow('Clear the book leg')
    expect(queryContract).not.toHaveBeenCalled()
  })
})
