import { describe, it, expect } from 'vitest'
import { lookupByDenom, lookupByCW20, lookupByTokenId, lookupByAssetInfo, TOKENS } from '../tokenRegistry'

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

describe('wrapped native token entries', () => {
  it('has LUNC-C entry', () => {
    const luncC = TOKENS.find((t) => t.symbol === 'LUNC-C')
    expect(luncC).toBeDefined()
    expect(luncC?.name).toBe('Wrapped Luna Classic')
    expect(luncC?.decimals).toBe(6)
  })

  it('has USTC-C entry', () => {
    const ustcC = TOKENS.find((t) => t.symbol === 'USTC-C')
    expect(ustcC).toBeDefined()
    expect(ustcC?.name).toBe('Wrapped TerraClassicUSD')
    expect(ustcC?.decimals).toBe(6)
  })
})
