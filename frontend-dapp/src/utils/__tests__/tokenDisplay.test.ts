import { describe, it, expect } from 'vitest'
import { shortenAddress, getTokenDisplaySymbol, isAddressLike, getAddressForBlockie } from '../tokenDisplay'

describe('shortenAddress', () => {
  it('returns short addresses unchanged', () => {
    expect(shortenAddress('terra1abc')).toBe('terra1abc')
  })

  it('truncates long addresses with defaults', () => {
    const addr = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
    const result = shortenAddress(addr)
    expect(result).toBe('terra16w…vhpax3')
    expect(result.length).toBeLessThan(addr.length)
  })

  it('respects custom start/end chars', () => {
    const addr = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
    const result = shortenAddress(addr, 6, 4)
    expect(result).toBe('terra1…pax3')
  })

  it('handles address exactly at threshold', () => {
    const addr = 'terra1abcdef1234'
    expect(shortenAddress(addr, 6, 4)).toBe('terra1…1234')
  })
})

describe('getTokenDisplaySymbol', () => {
  it('returns empty string for empty input', () => {
    expect(getTokenDisplaySymbol('')).toBe('')
  })

  it('returns registry symbol for known denom', () => {
    expect(getTokenDisplaySymbol('uluna')).toBe('LUNC')
  })

  it('returns registry symbol for known CW20', () => {
    expect(getTokenDisplaySymbol('terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3')).toBe('CL8Y')
  })

  it('returns tokenId for non-address strings', () => {
    expect(getTokenDisplaySymbol('FOO')).toBe('FOO')
  })
})

describe('isAddressLike', () => {
  it('recognizes terra addresses', () => {
    const addr = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
    expect(isAddressLike(addr)).toBe(true)
  })

  it('recognizes 0x addresses', () => {
    expect(isAddressLike('0x' + '0'.repeat(40))).toBe(true)
  })

  it('rejects short strings', () => {
    expect(isAddressLike('terra1short')).toBe(false)
  })
})

describe('getAddressForBlockie', () => {
  it('returns contract_addr for token AssetInfo', () => {
    const info = { token: { contract_addr: 'terra1abc' } }
    expect(getAddressForBlockie(info)).toBe('terra1abc')
  })

  it('returns undefined for native_token AssetInfo', () => {
    const info = { native_token: { denom: 'uluna' } }
    expect(getAddressForBlockie(info)).toBeUndefined()
  })
})
