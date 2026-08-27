import { describe, expect, it } from 'vitest'
import type { IndexerPair, PairInfo } from '@/types'
import {
  canonicalPairSymbol,
  compareHumanQuoteVolumeDesc,
  comparePairCatalog,
  firstCatalogPairAddress,
  firstEconomicIndexerPairAddress,
  firstUst1CustcPairAddress,
  resolveChartsHeroPairAddress,
  isGemSymbol,
  isTestPair,
  sortIndexerPairsByCatalog,
  sortPairInfosByCatalog,
} from '@/utils/pairCatalogRank'

const UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
const CUSTC = 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch'
const CLUNC = 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg'
const USTR = 'terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv'
const EMBER = 'terra1ember00000000000000000000000000000001'
const CORAL = 'terra1coral00000000000000000000000000000002'
const JADE = 'terra1jade000000000000000000000000000000003'

function pairInfo(addr: string, a: string, b: string): PairInfo {
  return {
    contract_addr: addr,
    liquidity_token: `${addr}lp`,
    asset_infos: [{ token: { contract_addr: a } }, { token: { contract_addr: b } }],
  }
}

function indexerPair(
  addr: string,
  s0: string,
  s1: string,
  a0: string,
  a1: string,
  volume: string,
  decimals1 = 6
): IndexerPair {
  return {
    pair_address: addr,
    asset_0: { symbol: s0, contract_addr: a0, denom: null, decimals: 6 },
    asset_1: { symbol: s1, contract_addr: a1, denom: null, decimals: decimals1 },
    lp_token: `${addr}lp`,
    fee_bps: 30,
    is_active: true,
    volume_quote_24h: volume,
  }
}

describe('canonicalPairSymbol / gems (GitLab #534)', () => {
  it('maps wrap and native aliases onto hubs', () => {
    expect(canonicalPairSymbol('cLUNC')).toBe('CLUNC')
    expect(canonicalPairSymbol('uluna')).toBe('CLUNC')
    expect(canonicalPairSymbol('cUSTC')).toBe('CUSTC')
    expect(canonicalPairSymbol('uusd')).toBe('CUSTC')
    expect(canonicalPairSymbol('TCL8Y')).toBe('CL8Y')
    expect(canonicalPairSymbol('UST1')).toBe('UST1')
  })

  it('recognizes faucet gems including LocalTerra extras', () => {
    expect(isGemSymbol('EMBER')).toBe(true)
    expect(isGemSymbol('QUARTZ')).toBe(true)
    expect(isGemSymbol('IRON')).toBe(true)
    expect(isGemSymbol('UST1')).toBe(false)
    expect(isGemSymbol('cLUNC')).toBe(false)
  })

  it('treats a pair as test when either leg is a gem (P534-1)', () => {
    expect(isTestPair('UST1', 'cUSTC')).toBe(false)
    expect(isTestPair('EMBER', 'CORAL')).toBe(true)
    expect(isTestPair('UST1', 'EMBER')).toBe(true)
  })
})

describe('sortIndexerPairsByCatalog (GitLab #534)', () => {
  it('lists economic UST1 markets together ahead of gem pairs (factory-creation order inverted)', () => {
    const ust1Ustr = indexerPair('terra1p-ustr', 'UST1', 'USTR', UST1, USTR, '0', 18)
    const ust1Custc = indexerPair('terra1p-custc', 'UST1', 'cUSTC', UST1, CUSTC, '0')
    const emberCoral = indexerPair('terra1p-ember', 'EMBER', 'CORAL', EMBER, CORAL, '999999')
    const cluncUst1 = indexerPair('terra1p-clunc', 'cLUNC', 'UST1', CLUNC, UST1, '0')
    const emberJade = indexerPair('terra1p-jade', 'EMBER', 'JADE', EMBER, JADE, '1')

    const sorted = sortIndexerPairsByCatalog([ust1Ustr, ust1Custc, emberCoral, emberJade, cluncUst1])
    expect(sorted.map((p) => p.pair_address)).toEqual([
      cluncUst1.pair_address,
      ust1Custc.pair_address,
      ust1Ustr.pair_address,
      emberCoral.pair_address,
      emberJade.pair_address,
    ])
  })

  it('sorts within a hub by human quote volume so 18-dec USTR does not always beat 6-dec cUSTC', () => {
    const lowUstr = indexerPair('terra1p-ustr', 'UST1', 'USTR', UST1, USTR, '1000000000000000000', 18) // 1 USTR
    const highCustc = indexerPair('terra1p-custc', 'UST1', 'cUSTC', UST1, CUSTC, '5000000', 6) // 5 cUSTC
    const sorted = sortIndexerPairsByCatalog([lowUstr, highCustc])
    expect(sorted[0].pair_address).toBe(highCustc.pair_address)
  })
})

describe('sortPairInfosByCatalog', () => {
  it('uses registry symbols so columbus-5 economic pairs rank without indexer metadata', () => {
    const gem = pairInfo('terra1p-gem', EMBER, CORAL)
    const ust1Custc = pairInfo('terra1p-econ', UST1, CUSTC)
    const sorted = sortPairInfosByCatalog([gem, ust1Custc])
    expect(sorted[0].contract_addr).toBe(ust1Custc.contract_addr)
    expect(firstCatalogPairAddress([gem, ust1Custc])).toBe(ust1Custc.contract_addr)
  })
})

describe('compareHumanQuoteVolumeDesc', () => {
  it('compares mixed-decimal raw amounts in human units', () => {
    expect(compareHumanQuoteVolumeDesc('1000000', 6, '1000000000000000000', 18)).toBe(0)
    expect(compareHumanQuoteVolumeDesc('2000000', 6, '1000000000000000000', 18)).toBe(-1)
  })
})

describe('Charts hero pick (GitLab #680)', () => {
  it('prefers UST1/cUSTC over cLUNC/UST1 and does not change Trade firstCatalogPairAddress', () => {
    const cluncUst1 = indexerPair('terra1p-clunc', 'cLUNC', 'UST1', CLUNC, UST1, '999')
    const ust1Custc = indexerPair('terra1p-hero', 'UST1', 'cUSTC', UST1, CUSTC, '0')
    const gem = indexerPair('terra1p-gem', 'EMBER', 'CORAL', EMBER, CORAL, '999999')
    expect(firstUst1CustcPairAddress([cluncUst1, gem, ust1Custc])).toBe(ust1Custc.pair_address)
    expect(resolveChartsHeroPairAddress([cluncUst1, gem, ust1Custc], 'local')).toBe(ust1Custc.pair_address)
    expect(firstEconomicIndexerPairAddress([cluncUst1])).toBe(cluncUst1.pair_address)
    expect(resolveChartsHeroPairAddress([cluncUst1], 'local')).toBe(cluncUst1.pair_address)
    expect(firstCatalogPairAddress([pairInfo('terra1p-trade', UST1, CUSTC)])).toBe('terra1p-trade')
  })
})

describe('comparePairCatalog', () => {
  it('ranks economic ahead of gems even when gem volume is huge', () => {
    const gem = {
      symbol0: 'EMBER',
      symbol1: 'CORAL',
      volume: { raw: '999999999999', quoteDecimals: 6 },
      address: 'g',
    }
    const econ = {
      symbol0: 'UST1',
      symbol1: 'cUSTC',
      volume: { raw: '1', quoteDecimals: 6 },
      address: 'e',
    }
    expect(comparePairCatalog(econ, gem)).toBeLessThan(0)
  })
})
