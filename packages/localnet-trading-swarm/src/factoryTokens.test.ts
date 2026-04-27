import { describe, it, expect } from 'vitest'
import { uniqueCw20TokenAddresses } from './factoryTokens.js'
import type { PairInfo } from './types.js'

describe('uniqueCw20TokenAddresses', () => {
  it('dedupes and sorts', () => {
    const pairs: PairInfo[] = [
      {
        asset_infos: [
          { token: { contract_addr: 'terra1bbb' } },
          { token: { contract_addr: 'terra1aaa' } },
        ],
        contract_addr: 'terra1pair1',
        liquidity_token: 'terra1lp1',
      },
      {
        asset_infos: [
          { token: { contract_addr: 'terra1aaa' } },
          { native_token: { denom: 'uluna' } },
        ],
        contract_addr: 'terra1pair2',
        liquidity_token: 'terra1lp2',
      },
    ]
    expect(uniqueCw20TokenAddresses(pairs)).toEqual(['terra1aaa', 'terra1bbb'])
  })
})
