import { describe, it, expect } from 'vitest'
import {
  PAIR_DISCOUNT_REGISTRY_STORAGE_KEY,
  advertisedDiscountBps,
  decodeCwStoragePlusOptionalAddr,
  normalizeTerraAddr,
  pairDiscountRegistryRawKeyB64,
  pairFeeDiscountApplies,
  parseGetDiscountRegistryResponse,
} from '../pairDiscountRegistry'

const REGISTRY = 'terra1wcczsdk7jwj99n3my6wx8wr4ee0hn6yaapgd792lgx5elrdtrn2scfnecz'
const OTHER = 'terra1otherdiscountregistry0000000000000000000000000001'

describe('pairDiscountRegistry (GitLab #537 / I14)', () => {
  it('raw key b64 is btoa(discount_registry)', () => {
    expect(pairDiscountRegistryRawKeyB64()).toBe(btoa(PAIR_DISCOUNT_REGISTRY_STORAGE_KEY))
  })

  it('decodeCwStoragePlusOptionalAddr reads JSON null as unwired', () => {
    expect(decodeCwStoragePlusOptionalAddr(btoa('null'))).toBeNull()
    expect(decodeCwStoragePlusOptionalAddr(null)).toBeNull()
    expect(decodeCwStoragePlusOptionalAddr('')).toBeNull()
  })

  it('decodeCwStoragePlusOptionalAddr reads JSON string Addr', () => {
    expect(decodeCwStoragePlusOptionalAddr(btoa(JSON.stringify(REGISTRY)))).toBe(REGISTRY)
  })

  it('pairFeeDiscountApplies requires matching configured registry', () => {
    expect(pairFeeDiscountApplies(null, REGISTRY)).toBe(false)
    expect(pairFeeDiscountApplies(undefined, REGISTRY)).toBe(false)
    expect(pairFeeDiscountApplies(REGISTRY, '')).toBe(false)
    expect(pairFeeDiscountApplies(OTHER, REGISTRY)).toBe(false)
    expect(pairFeeDiscountApplies(REGISTRY, REGISTRY)).toBe(true)
    expect(pairFeeDiscountApplies(REGISTRY.toUpperCase(), REGISTRY)).toBe(true)
  })

  it('advertisedDiscountBps is 0 on unwired or mismatched pair', () => {
    expect(advertisedDiscountBps(9500, null, REGISTRY)).toBe(0)
    expect(advertisedDiscountBps(9500, OTHER, REGISTRY)).toBe(0)
    expect(advertisedDiscountBps(9500, REGISTRY, REGISTRY)).toBe(9500)
    expect(advertisedDiscountBps(0, REGISTRY, REGISTRY)).toBe(0)
  })

  it('parseGetDiscountRegistryResponse accepts discount_registry field', () => {
    expect(parseGetDiscountRegistryResponse({ discount_registry: REGISTRY })).toBe(REGISTRY)
    expect(parseGetDiscountRegistryResponse({ discount_registry: null })).toBeNull()
    expect(parseGetDiscountRegistryResponse({ registry: REGISTRY })).toBe(REGISTRY)
  })

  it('normalizeTerraAddr rejects junk', () => {
    expect(normalizeTerraAddr('not-an-addr')).toBeNull()
    expect(normalizeTerraAddr('terra1')).toBeNull()
    expect(normalizeTerraAddr('terra1feediscount')).toBe('terra1feediscount')
    expect(normalizeTerraAddr(REGISTRY)).toBe(REGISTRY)
  })
})
