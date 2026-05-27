import { describe, expect, it, vi } from 'vitest'
import { resolvePairDecimalsForLimitPriceRefFromChain } from '../limitOrderPriceReference'
import type { PairInfo } from '@/types'

vi.mock('@/utils/tokenDisplay', () => ({
  fetchCW20TokenInfo: vi.fn(async (addr: string) => {
    if (addr === 'terra1ember') return { symbol: 'EMBER', name: 'Ember', decimals: 6, total_supply: '0' }
    if (addr === 'terra1coral') return { symbol: 'CORAL', name: 'Coral', decimals: 6, total_supply: '0' }
    return null
  }),
}))

describe('resolvePairDecimalsForLimitPriceRefFromChain', () => {
  const pairInfo: PairInfo = {
    contract_addr: 'pair1',
    liquidity_token: 'lp',
    asset_infos: [{ token: { contract_addr: 'terra1ember' } }, { token: { contract_addr: 'terra1coral' } }],
  }

  it('resolves decimals from on-chain CW20 token_info when registry lacks the pair', async () => {
    const dec = await resolvePairDecimalsForLimitPriceRefFromChain(pairInfo)
    expect(dec).toEqual({ d0: 6, d1: 6 })
  })
})
