import { describe, it, expect } from 'vitest'
import { isValidTerraAddress, GAS_PRICE_ULUNA } from './constants'

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
