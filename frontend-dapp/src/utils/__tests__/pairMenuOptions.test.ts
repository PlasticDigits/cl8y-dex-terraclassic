import { describe, expect, it } from 'vitest'
import type { IndexerPair, PairInfo } from '@/types'
import { indexerPairToPairInfo } from '@/types'
import {
  indexerPairsToMenuSelectOptions,
  pairInfoMenuLabel,
  pairInfosToMenuSelectOptions,
} from '@/utils/pairMenuOptions'

describe('pairInfoMenuLabel', () => {
  it('joins display symbols and shortened pair address', () => {
    const pair: PairInfo = {
      contract_addr: 'terra1pairqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq',
      liquidity_token: 'terra1lp',
      asset_infos: [
        { token: { contract_addr: 'terra1tokenaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' } },
        { native_token: { denom: 'uluna' } },
      ],
    }
    const label = pairInfoMenuLabel(pair)
    expect(label).toContain('—')
    expect(label).toContain('terra1pa')
  })
})

describe('indexerPairsToMenuSelectOptions', () => {
  it('maps indexer pairs to value/label rows', () => {
    const pairs: IndexerPair[] = [
      {
        pair_address: 'terra1pairqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq',
        asset_0: {
          symbol: 'A',
          contract_addr: 'terra1tokenaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
          denom: null,
          decimals: 6,
        },
        asset_1: { symbol: 'B', contract_addr: null, denom: 'uluna', decimals: 6 },
        lp_token: 'terra1lp',
        fee_bps: 30,
        is_active: true,
      },
    ]
    const opts = indexerPairsToMenuSelectOptions(pairs)
    expect(opts).toHaveLength(1)
    expect(opts[0].value).toBe(pairs[0].pair_address)
    expect(opts[0].label).toBe(pairInfoMenuLabel(indexerPairToPairInfo(pairs[0])))
  })

  it('returns [] when there are no pairs', () => {
    expect(indexerPairsToMenuSelectOptions([])).toEqual([])
  })
})

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
