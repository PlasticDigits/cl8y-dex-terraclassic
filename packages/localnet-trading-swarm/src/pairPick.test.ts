import { describe, expect, it } from 'vitest'
import {
  cw20AddrsFromPairs,
  filterGemPairs,
  filterTaxPairs,
  findTaxInclusiveRoute,
  pairTouchesTax,
  randomCw20PairEndpoints,
} from './pairPick.js'
import type { PairInfo } from './types.js'
import { tokenAssetInfo } from './types.js'

function pair(a: string, b: string, addr = `terra1p${a.slice(-2)}${b.slice(-2)}`): PairInfo {
  return {
    asset_infos: [tokenAssetInfo(a), tokenAssetInfo(b)],
    contract_addr: addr,
    liquidity_token: `${addr}lp`,
  }
}

const TAX = 'terra1taxaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
const EMBER = 'terra1emberaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
const CORAL = 'terra1coralaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
const ONYX = 'terra1onyxaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'

const taxEmber = pair(TAX, EMBER, 'terra1taxember')
const emberCoral = pair(EMBER, CORAL, 'terra1embercoral')
const onyxCoral = pair(ONYX, CORAL, 'terra1onyxcoral')
const all = [taxEmber, emberCoral, onyxCoral]
const taxSet = new Set([TAX])

describe('filterGemPairs / filterTaxPairs', () => {
  it('drops tax/EMBER from gem workers', () => {
    const gems = filterGemPairs(all, taxSet)
    expect(gems.map((p) => p.contract_addr)).toEqual(['terra1embercoral', 'terra1onyxcoral'])
    expect(gems.some((p) => pairTouchesTax(p, taxSet))).toBe(false)
  })

  it('tax workers only see the tax pair', () => {
    expect(filterTaxPairs(all, taxSet).map((p) => p.contract_addr)).toEqual(['terra1taxember'])
  })

  it('empty tax set leaves gems unchanged and tax filter empty', () => {
    expect(filterGemPairs(all, new Set()).length).toBe(3)
    expect(filterTaxPairs(all, new Set())).toEqual([])
  })
})

describe('randomCw20PairEndpoints exclude', () => {
  it('never offers the tax token when excluded', () => {
    for (let i = 0; i < 40; i++) {
      const e = randomCw20PairEndpoints(all, taxSet)
      expect(e).not.toBeNull()
      expect(e!.from).not.toBe(TAX)
      expect(e!.to).not.toBe(TAX)
    }
  })

  it('cw20AddrsFromPairs excludes tax', () => {
    expect(cw20AddrsFromPairs(all, taxSet).sort()).toEqual([CORAL, EMBER, ONYX].sort())
  })
})

describe('findTaxInclusiveRoute', () => {
  it('finds TAX→EMBER→CORAL (≥2 hops) when selling tax', () => {
    const got = findTaxInclusiveRoute(all, taxSet, true)
    expect(got).not.toBeNull()
    expect(got!.from).toBe(TAX)
    expect(got!.route.length).toBeGreaterThanOrEqual(2)
  })

  it('finds a ≥2hop that ends at tax when buying', () => {
    const got = findTaxInclusiveRoute(all, taxSet, false)
    expect(got).not.toBeNull()
    expect(got!.to).toBe(TAX)
    expect(got!.route.length).toBeGreaterThanOrEqual(2)
  })

  it('returns null without a tax token on the graph', () => {
    expect(findTaxInclusiveRoute([emberCoral], taxSet, true)).toBeNull()
  })
})
