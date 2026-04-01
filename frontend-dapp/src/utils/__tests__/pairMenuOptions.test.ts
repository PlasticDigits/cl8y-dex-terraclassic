import { describe, expect, it } from 'vitest'
import type { PairInfo } from '@/types'
import { pairInfosToMenuSelectOptions } from '@/utils/pairMenuOptions'

describe('pairInfosToMenuSelectOptions', () => {
  it('prepends placeholder and maps pair contract + symbols', () => {
    const pairs: PairInfo[] = [
      {
        contract_addr: 'terra1pairqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq',
        liquidity_token: 'terra1lpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq',
        asset_infos: [
          { token: { contract_addr: 'terra1tokenaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' } },
          { native_token: { denom: 'uluna' } },
        ],
      },
    ]
    const opts = pairInfosToMenuSelectOptions(pairs)
    expect(opts[0]).toEqual({ value: '', label: 'Select pair…' })
    expect(opts[1].value).toBe(pairs[0].contract_addr)
    expect(opts[1].label).toMatch(/\s\/\s/) /* token A / token B */
    expect(opts[1].label).toContain('—')
    expect(opts[1].label).toContain('terra1pa')
  })

  it('returns no rows when there are no pairs', () => {
    expect(pairInfosToMenuSelectOptions([])).toEqual([])
  })

  it('accepts custom placeholder', () => {
    const pairs: PairInfo[] = [
      {
        contract_addr: 'terra1pairzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz',
        liquidity_token: 'terra1lpzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz',
        asset_infos: [{ native_token: { denom: 'uusd' } }, { native_token: { denom: 'uluna' } }],
      },
    ]
    const opts = pairInfosToMenuSelectOptions(pairs, { placeholder: { value: '', label: 'Pick…' } })
    expect(opts[0]).toEqual({ value: '', label: 'Pick…' })
    expect(opts).toHaveLength(2)
  })
})
