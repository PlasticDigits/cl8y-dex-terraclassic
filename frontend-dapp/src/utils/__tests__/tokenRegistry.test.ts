import { describe, it, expect } from 'vitest'
import {
  lookupByDenom,
  lookupByCW20,
  lookupByTokenId,
  lookupByAssetInfo,
  registryProductSymbol,
  TOKENS,
} from '../tokenRegistry'

describe('lookupByDenom', () => {
  it('returns LUNC for uluna', () => {
    const entry = lookupByDenom('uluna')
    expect(entry).toBeDefined()
    expect(entry!.symbol).toBe('LUNC')
    expect(entry!.decimals).toBe(6)
  })

  it('returns USTC for uusd', () => {
    const entry = lookupByDenom('uusd')
    expect(entry).toBeDefined()
    expect(entry!.symbol).toBe('USTC')
  })

  it('is case-insensitive', () => {
    expect(lookupByDenom('ULUNA')).toBeDefined()
  })

  it('returns undefined for unknown denom', () => {
    expect(lookupByDenom('unknown')).toBeUndefined()
  })
})

describe('lookupByCW20', () => {
  it('returns CL8Y for its contract address', () => {
    const entry = lookupByCW20('terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3')
    expect(entry).toBeDefined()
    expect(entry!.symbol).toBe('CL8Y')
    // Mainnet CL8Y / LocalTerra TCL8Y are 18 decimals (GitLab #476 / #383).
    expect(entry!.decimals).toBe(18)
  })

  it('returns USTR for its contract address', () => {
    const entry = lookupByCW20('terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv')
    expect(entry).toBeDefined()
    expect(entry!.symbol).toBe('USTR')
    expect(entry!.decimals).toBe(18)
  })

  it('is case-insensitive', () => {
    expect(lookupByCW20('TERRA16WTML2Q66G82FDKX66TAP0QJKAHQWP4LWQ3NGTYGACG5Q0KZYCGQVHPAX3')).toBeDefined()
  })

  it('returns undefined for unknown address', () => {
    expect(lookupByCW20('terra1unknown')).toBeUndefined()
  })
})

describe('lookupByTokenId', () => {
  it('resolves native denoms', () => {
    expect(lookupByTokenId('uluna')?.symbol).toBe('LUNC')
  })

  it('resolves CW20 addresses', () => {
    expect(lookupByTokenId('terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3')?.symbol).toBe('CL8Y')
  })

  it('returns undefined for unknown', () => {
    expect(lookupByTokenId('xyz')).toBeUndefined()
  })
})

describe('registryProductSymbol (GitLab #630)', () => {
  it('maps known natives case-insensitively', () => {
    expect(registryProductSymbol('uluna')).toBe('LUNC')
    expect(registryProductSymbol('ULUNA')).toBe('LUNC')
    expect(registryProductSymbol('uusd')).toBe('USTC')
    expect(registryProductSymbol('UUSD')).toBe('USTC')
  })

  it('maps wrap CW20s to cLUNC / cUSTC', () => {
    expect(registryProductSymbol('terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg')).toBe('cLUNC')
    expect(registryProductSymbol('terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch')).toBe('cUSTC')
  })

  it('returns undefined for unknown natives and empty input', () => {
    expect(registryProductSymbol('ufoo')).toBeUndefined()
    expect(registryProductSymbol('ibc/ABC')).toBeUndefined()
    expect(registryProductSymbol('')).toBeUndefined()
    expect(registryProductSymbol(null)).toBeUndefined()
  })
})

describe('lookupByAssetInfo', () => {
  it('resolves CW20 AssetInfo', () => {
    const info = { token: { contract_addr: 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3' } }
    expect(lookupByAssetInfo(info)?.symbol).toBe('CL8Y')
  })

  it('resolves native AssetInfo', () => {
    const info = { native_token: { denom: 'uluna' } }
    expect(lookupByAssetInfo(info)?.symbol).toBe('LUNC')
  })

  it('returns undefined for unknown AssetInfo', () => {
    const info = { token: { contract_addr: 'terra1unknown' } }
    expect(lookupByAssetInfo(info)).toBeUndefined()
  })
})

describe('wrapped native token entries (GitLab #507)', () => {
  it('has cLUNC entry (not LUNC-C) with distinct CLUNC badge logo', () => {
    const luncC = TOKENS.find((t) => t.symbol === 'cLUNC')
    expect(luncC).toBeDefined()
    expect(luncC?.name).toBe('Wrapped Luna Classic')
    expect(luncC?.decimals).toBe(6)
    expect(luncC?.logoURI).toContain('/tokenlist/images/CLUNC.png')
    expect(TOKENS.find((t) => t.symbol === 'LUNC-C')).toBeUndefined()
  })

  it('has cUSTC entry (not USTC-C) with distinct CUSTC badge logo', () => {
    const ustcC = TOKENS.find((t) => t.symbol === 'cUSTC')
    expect(ustcC).toBeDefined()
    expect(ustcC?.name).toBe('Wrapped TerraClassicUSD')
    expect(ustcC?.decimals).toBe(6)
    expect(ustcC?.logoURI).toContain('/tokenlist/images/CUSTC.png')
    expect(TOKENS.find((t) => t.symbol === 'USTC-C')).toBeUndefined()
  })
})
