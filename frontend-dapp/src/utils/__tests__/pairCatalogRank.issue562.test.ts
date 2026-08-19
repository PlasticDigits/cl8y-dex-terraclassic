import { afterEach, describe, expect, it, vi } from 'vitest'
import type { PairInfo } from '@/types'
import {
  COLUMBUS5_GEM_ADDRESSES,
  defaultRetailSwapTokenPair,
  filterRetailDiscoveryPairInfos,
  filterRetailDiscoveryTokens,
  firstCatalogPairAddress,
  isEconomicHubTokenId,
  isGemTokenId,
  retailExposeTestTokens,
  shouldRejectGemBridgeQuote,
} from '@/utils/pairCatalogRank'

const UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
const CUSTC = 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch'
const CL8Y = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const RUBY = 'terra1fga508hzx8dd7x8q4uhm6mdhkqv6fxrtsea3r27smdqmv5k2jgxq5zk9fc'
const QUARTZ = 'terra17dpnjlpgsnm8muu4msfjra4f2hrptnjp2jdpkka4p0e3px42ayxq0pmc2z'
const PEARL = 'terra18fzufz8cs7ez49xjwgs248x85za5v50yug55fj7lyxp9hapxyr7qnh3czs'
const EMBER = 'terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94'
const LOCAL_EMBER = 'terra1ember00000000000000000000000000000001'

function pairInfo(addr: string, a: string, b: string): PairInfo {
  return {
    contract_addr: addr,
    liquidity_token: `${addr}lp`,
    asset_infos: [{ token: { contract_addr: a } }, { token: { contract_addr: b } }],
  }
}

describe('retailExposeTestTokens (GitLab #562 U1)', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
  })

  it('is false on mainnet when the override is unset', () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.stubEnv('VITE_SHOW_TEST_TOKENS', '')
    expect(retailExposeTestTokens()).toBe(false)
  })

  it('is true on local', () => {
    vi.stubEnv('VITE_NETWORK', 'local')
    expect(retailExposeTestTokens()).toBe(true)
  })

  it('is true on mainnet when VITE_SHOW_TEST_TOKENS=true (A8)', () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.stubEnv('VITE_SHOW_TEST_TOKENS', 'true')
    expect(retailExposeTestTokens()).toBe(true)
  })
})

describe('columbus-5 gem address set (U2 / A11 / X1)', () => {
  it('classifies all eight mainnet gem addrs as gems even without VITE_TOKEN_*', () => {
    expect(COLUMBUS5_GEM_ADDRESSES.size).toBe(8)
    expect(isGemTokenId(EMBER)).toBe(true)
    expect(isGemTokenId(RUBY)).toBe(true)
    expect(isGemTokenId(QUARTZ)).toBe(true)
    expect(isGemTokenId(PEARL)).toBe(true)
  })

  it('never classifies UST1 / wrap / CL8Y as gems (U6 / P534-8)', () => {
    expect(isGemTokenId(UST1)).toBe(false)
    expect(isGemTokenId(CUSTC)).toBe(false)
    expect(isGemTokenId(CL8Y)).toBe(false)
    expect(isGemTokenId('uluna')).toBe(false)
    expect(isGemTokenId('uusd')).toBe(false)
    expect(isEconomicHubTokenId(UST1)).toBe(true)
    expect(isEconomicHubTokenId(CL8Y)).toBe(true)
  })

  it('hides a gem contract even if display symbol is spoofed as UST1 (X1)', () => {
    expect(isGemTokenId(RUBY)).toBe(true)
    expect(isEconomicHubTokenId(RUBY)).toBe(false)
  })

  it('does not hide a listed economic token if someone named it RUBY (X1)', () => {
    expect(isGemTokenId(CL8Y)).toBe(false)
    expect(isEconomicHubTokenId(CL8Y)).toBe(true)
  })
})

describe('production discovery filters (A1 / A3 / A4)', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
  })

  it('omits gems from token browse on mainnet and keeps wrap/hubs', () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.stubEnv('VITE_SHOW_TEST_TOKENS', '')
    const mixed = [RUBY, UST1, EMBER, 'uluna', CUSTC, QUARTZ]
    const filtered = filterRetailDiscoveryTokens(mixed)
    expect(filtered).toEqual([UST1, 'uluna', CUSTC])
    expect(filtered.some((t) => t === RUBY || t === QUARTZ || t === EMBER)).toBe(false)
  })

  it('keeps gems on local (A7 / U4)', () => {
    vi.stubEnv('VITE_NETWORK', 'local')
    const mixed = [RUBY, UST1, LOCAL_EMBER]
    expect(filterRetailDiscoveryTokens(mixed)).toEqual(mixed)
  })

  it('omits gem pairs from production pair browse', () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.stubEnv('VITE_SHOW_TEST_TOKENS', '')
    const gem = pairInfo('terra1p-gem', EMBER, RUBY)
    const econ = pairInfo('terra1p-ust', UST1, CUSTC)
    expect(filterRetailDiscoveryPairInfos([gem, econ]).map((p) => p.contract_addr)).toEqual([econ.contract_addr])
    expect(firstCatalogPairAddress([gem, econ])).toBe(econ.contract_addr)
  })

  it('defaults Swap pay/receive to economic tokens when gems lead the factory set (U8)', () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.stubEnv('VITE_SHOW_TEST_TOKENS', '')
    const pair = defaultRetailSwapTokenPair([RUBY, QUARTZ, UST1, CUSTC, 'uluna'])
    expect(pair).toEqual([UST1, CUSTC])
  })

  it('keeps factory/wrap order when choosing defaults (do not re-sort hubs)', () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.stubEnv('VITE_SHOW_TEST_TOKENS', '')
    expect(defaultRetailSwapTokenPair(['uluna', CUSTC, UST1, RUBY])).toEqual(['uluna', CUSTC])
  })
})

describe('gem-bridge quote reject (A10 / U9)', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
  })

  it('rejects economic→economic routes that hop a gem on production', () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.stubEnv('VITE_SHOW_TEST_TOKENS', '')
    expect(shouldRejectGemBridgeQuote(UST1, CUSTC, [UST1, RUBY, CUSTC])).toBe(true)
    expect(shouldRejectGemBridgeQuote(UST1, CUSTC, [UST1, CUSTC])).toBe(false)
  })

  it('allows gem hops on local and for gem exit-hatch endpoints', () => {
    vi.stubEnv('VITE_NETWORK', 'local')
    expect(shouldRejectGemBridgeQuote(UST1, CUSTC, [UST1, RUBY, CUSTC])).toBe(false)
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.stubEnv('VITE_SHOW_TEST_TOKENS', '')
    expect(shouldRejectGemBridgeQuote(RUBY, PEARL, [RUBY, QUARTZ, PEARL])).toBe(false)
  })
})
