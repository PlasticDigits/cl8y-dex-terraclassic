import { describe, it, expect } from 'vitest'
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
