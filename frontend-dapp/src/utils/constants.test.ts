import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  isValidTerraAddress,
  GAS_PRICE_ULUNA,
  WRAP_GAS_LIMIT,
  UNWRAP_GAS_LIMIT,
  NATIVE_WRAPPED_PAIRS,
  WRAPPED_NATIVE_PAIRS,
} from './constants'

describe('isValidTerraAddress', () => {
  it('accepts valid terra addresses', () => {
    expect(isValidTerraAddress('terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v')).toBe(true)
    expect(isValidTerraAddress('terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3')).toBe(true)
  })

  it('rejects invalid addresses', () => {
    expect(isValidTerraAddress('')).toBe(false)
    expect(isValidTerraAddress('cosmos1abcdef')).toBe(false)
    expect(isValidTerraAddress('terra1')).toBe(false)
    expect(isValidTerraAddress('TERRA1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v')).toBe(false)
  })
})

describe('GAS_PRICE_ULUNA', () => {
  it('has a default value', () => {
    expect(GAS_PRICE_ULUNA).toBeDefined()
    expect(parseFloat(GAS_PRICE_ULUNA)).toBeGreaterThan(0)
  })
})

describe('effectiveGasPriceUluna (GitLab #127)', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
  })

  it('floors a too-low VITE_GAS_PRICE_ULUNA so allowance+limit txs meet minimum fees', async () => {
    vi.stubEnv('VITE_GAS_PRICE_ULUNA', '0.015')
    vi.resetModules()
    const { effectiveGasPriceUluna, MIN_GAS_PRICE_ULUNA } = await import('./constants')
    expect(effectiveGasPriceUluna()).toBe(MIN_GAS_PRICE_ULUNA)
  })

  it('preserves values above the floor', async () => {
    vi.stubEnv('VITE_GAS_PRICE_ULUNA', '50')
    vi.resetModules()
    const { effectiveGasPriceUluna } = await import('./constants')
    expect(effectiveGasPriceUluna()).toBe(50)
  })
})

describe('wrap constants', () => {
  it('WRAP_GAS_LIMIT is positive', () => {
    expect(WRAP_GAS_LIMIT).toBeGreaterThan(0)
  })

  it('UNWRAP_GAS_LIMIT is positive', () => {
    expect(UNWRAP_GAS_LIMIT).toBeGreaterThan(0)
  })

  it('NATIVE_WRAPPED_PAIRS contains uluna and uusd', () => {
    expect('uluna' in NATIVE_WRAPPED_PAIRS).toBe(true)
    expect('uusd' in NATIVE_WRAPPED_PAIRS).toBe(true)
  })

  it('WRAPPED_NATIVE_PAIRS is consistent with NATIVE_WRAPPED_PAIRS', () => {
    for (const [native, wrapped] of Object.entries(NATIVE_WRAPPED_PAIRS)) {
      if (wrapped) {
        expect(WRAPPED_NATIVE_PAIRS[wrapped]).toBe(native)
      }
    }
  })
})

describe('isNativeWrapEnabled', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
  })

  it('is true only when mapper, treasury, and both wrap CW20s are set', async () => {
    vi.stubEnv('VITE_WRAP_MAPPER_ADDRESS', 'terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2')
    vi.stubEnv('VITE_TREASURY_ADDRESS', 'terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2')
    vi.stubEnv('VITE_LUNC_C_TOKEN_ADDRESS', 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg')
    vi.stubEnv('VITE_USTC_C_TOKEN_ADDRESS', 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch')
    vi.resetModules()
    const { isNativeWrapEnabled } = await import('./constants')
    expect(isNativeWrapEnabled()).toBe(true)
  })

  it('is false when any wrap address is missing', async () => {
    vi.stubEnv('VITE_WRAP_MAPPER_ADDRESS', 'terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2')
    vi.stubEnv('VITE_TREASURY_ADDRESS', '')
    vi.stubEnv('VITE_LUNC_C_TOKEN_ADDRESS', 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg')
    vi.stubEnv('VITE_USTC_C_TOKEN_ADDRESS', 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch')
    vi.resetModules()
    const { isNativeWrapEnabled } = await import('./constants')
    expect(isNativeWrapEnabled()).toBe(false)
  })
})
