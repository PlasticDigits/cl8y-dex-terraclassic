import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, it, expect, afterEach, vi } from 'vitest'
import {
  buildCreatePairCw20Options,
  getCreatePairCw20Options,
  listedCreatePairAddress,
  sameCreatePairAddress,
} from '@/utils/createPairTokenCatalog'
import { CL8Y_TOKEN_ADDRESS, LUNC_C_TOKEN_ADDRESS } from '@/utils/constants'
import { lookupByCW20 } from '@/utils/tokenRegistry'

const CL8Y = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const USTR = 'terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv'
const CLUNC_MAINNET = 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg'
const LOCAL_CLUNC = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const GEM = 'terra1yyca08xqdgvjz0psg56z67ejh9xms6l436u8y58m82npdqqhmmtqzjqhh0'

const PUBLISHED_CW20 = [
  { symbol: 'LUNC', name: 'Terra Luna Classic', denom: 'uluna', type: 'native' },
  { symbol: 'USTC', name: 'TerraClassicUSD', denom: 'uusd', type: 'native' },
  { symbol: 'cLUNC', name: 'Wrapped Luna Classic', address: CLUNC_MAINNET, type: 'cw20' },
  { symbol: 'CL8Y', name: 'CL8Y Token', address: CL8Y, type: 'cw20' },
  { symbol: 'USTR', name: 'USTR Token', address: USTR, type: 'cw20' },
  { symbol: 'EMPTY', name: 'Empty', address: '', type: 'cw20' },
  { symbol: 'BAD', name: 'Bad', address: 'terra1notvalid', type: 'cw20' },
  { symbol: 'DUP', name: 'Duplicate CL8Y', address: CL8Y.toUpperCase(), type: 'cw20' },
]

describe('buildCreatePairCw20Options (GitLab #542)', () => {
  it('T1: includes published CW20s and excludes natives', () => {
    const rows = buildCreatePairCw20Options({
      tokenlistTokens: PUBLISHED_CW20,
      overlays: {},
      gems: [],
    })
    const symbols = rows.map((r) => r.symbol)
    const addrs = rows.map((r) => r.address.toLowerCase())
    expect(symbols).toContain('cLUNC')
    expect(symbols).toContain('CL8Y')
    expect(symbols).toContain('USTR')
    expect(symbols).not.toContain('LUNC')
    expect(symbols).not.toContain('USTC')
    expect(addrs.some((a) => a === 'uluna' || a === 'uusd')).toBe(false)
  })

  it('T2: env overlay replaces the published mainnet address', () => {
    const rows = buildCreatePairCw20Options({
      tokenlistTokens: PUBLISHED_CW20,
      overlays: { cLUNC: LOCAL_CLUNC },
      gems: [],
    })
    const clunc = rows.find((r) => r.symbol === 'cLUNC')
    expect(clunc?.address).toBe(LOCAL_CLUNC)
    expect(rows.some((r) => r.address.toLowerCase() === CLUNC_MAINNET)).toBe(false)
  })

  it('T3: empty overlay keeps the published mainnet address', () => {
    const rows = buildCreatePairCw20Options({
      tokenlistTokens: PUBLISHED_CW20,
      overlays: { cLUNC: '', UST1: '   ' },
      gems: [],
    })
    expect(rows.find((r) => r.symbol === 'cLUNC')?.address).toBe(CLUNC_MAINNET)
  })

  it('T4: gems appear only when provided (soft-launch env)', () => {
    const without = buildCreatePairCw20Options({
      tokenlistTokens: PUBLISHED_CW20,
      overlays: {},
      gems: [],
    })
    expect(without.some((r) => r.symbol === 'EMBER')).toBe(false)

    const withGem = buildCreatePairCw20Options({
      tokenlistTokens: PUBLISHED_CW20,
      overlays: {},
      gems: [{ symbol: 'EMBER', address: GEM }],
    })
    expect(withGem.some((r) => r.symbol === 'EMBER' && r.address === GEM)).toBe(true)
  })

  it('T5: drops empty and invalid bech32 rows', () => {
    const rows = buildCreatePairCw20Options({
      tokenlistTokens: PUBLISHED_CW20,
      overlays: { CL8Y: 'uluna' },
      gems: [{ symbol: 'BADGEM', address: '' }],
    })
    expect(rows.some((r) => r.symbol === 'EMPTY' || r.symbol === 'BAD' || r.symbol === 'BADGEM')).toBe(false)
    expect(rows.some((r) => r.symbol === 'CL8Y')).toBe(false)
    expect(rows.every((r) => r.address.length > 0)).toBe(true)
  })

  it('T6: dedupes by lowercase address', () => {
    const rows = buildCreatePairCw20Options({
      tokenlistTokens: PUBLISHED_CW20,
      overlays: {},
      gems: [{ symbol: 'CL8Y-AGAIN', address: CL8Y }],
    })
    const cl8yHits = rows.filter((r) => r.address.toLowerCase() === CL8Y)
    expect(cl8yHits).toHaveLength(1)
  })
})

describe('getCreatePairCw20Options live catalog', () => {
  it('never offers native denoms and always includes CL8Y when the address resolves', () => {
    const rows = getCreatePairCw20Options()
    const addrs = rows.map((r) => r.address.toLowerCase())
    expect(addrs).not.toContain('uluna')
    expect(addrs).not.toContain('uusd')
    expect(rows.some((r) => r.symbol === 'LUNC' || r.symbol === 'USTC')).toBe(false)
    expect(rows.some((r) => r.address.toLowerCase() === CL8Y_TOKEN_ADDRESS.toLowerCase())).toBe(true)
  })

  it('A14 / C5: env overlay wins for cLUNC when VITE_LUNC_C_TOKEN_ADDRESS is set', () => {
    const rows = getCreatePairCw20Options()
    const clunc = rows.find((r) => r.symbol === 'cLUNC')
    if (!LUNC_C_TOKEN_ADDRESS) {
      expect(clunc?.address.toLowerCase()).toBe(CLUNC_MAINNET)
      return
    }
    expect(clunc?.address.toLowerCase()).toBe(LUNC_C_TOKEN_ADDRESS.toLowerCase())
    if (LUNC_C_TOKEN_ADDRESS.toLowerCase() !== CLUNC_MAINNET) {
      expect(rows.some((r) => r.address.toLowerCase() === CLUNC_MAINNET)).toBe(false)
    }
  })

  it('published tokenlist CW20 addresses match registry display map (no address drift)', () => {
    const here = dirname(fileURLToPath(import.meta.url))
    const raw = readFileSync(join(here, '../../../../tokenlist/tokenlist.json'), 'utf8')
    const tokenlist = JSON.parse(raw) as {
      tokens: { type: string; address?: string; symbol: string }[]
    }
    for (const token of tokenlist.tokens) {
      if (token.type !== 'cw20' || !token.address) continue
      const reg = lookupByCW20(token.address)
      expect(reg, `registry missing ${token.symbol} ${token.address}`).toBeDefined()
      expect(reg!.symbol).toBe(token.symbol)
    }
  })
})

describe('create pair address helpers', () => {
  it('A8: sameCreatePairAddress is case-insensitive', () => {
    expect(sameCreatePairAddress(CL8Y, CL8Y.toUpperCase())).toBe(true)
    expect(sameCreatePairAddress(CL8Y, USTR)).toBe(false)
    expect(sameCreatePairAddress('', '')).toBe(false)
  })

  it('A10: listedCreatePairAddress ignores ids not in the catalog', () => {
    expect(listedCreatePairAddress([CL8Y], USTR)).toBeUndefined()
    expect(listedCreatePairAddress([CL8Y], CL8Y.toUpperCase())).toBe(CL8Y)
  })
})

describe('Create Pair gems gated on retailExposeTestTokens (GitLab #562 U7 / A5)', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
  })

  it('live catalog does not append mintable gems on a mainnet build', () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.stubEnv('VITE_SHOW_TEST_TOKENS', '')
    const rows = getCreatePairCw20Options()
    expect(rows.some((r) => r.symbol === 'EMBER' || r.symbol === 'RUBY')).toBe(false)
    expect(rows.some((r) => r.address.toLowerCase() === CL8Y_TOKEN_ADDRESS.toLowerCase())).toBe(true)
  })
})
