import { describe, it, expect } from 'vitest'
import {
  shortenAddress,
  shortenTraderAddress,
  TRADER_ADDR_END_CHARS,
  TRADER_ADDR_START_CHARS,
  getTokenDisplaySymbol,
  isAddressLike,
  getAddressForBlockie,
  usablePoolAssetName,
  formatPoolAssetFieldLabel,
  poolProvideAmountAriaLabel,
} from '../tokenDisplay'

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

  it('keeps 8/6 defaults (not trader 4/6)', () => {
    const addr = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
    expect(shortenAddress(addr)).toBe('terra16w…vhpax3')
    expect(shortenAddress(addr)).not.toBe(shortenTraderAddress(addr))
  })
})

describe('shortenTraderAddress (GitLab #656)', () => {
  const addr = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'

  it('uses 4/6 (terr… + last 6)', () => {
    expect(TRADER_ADDR_START_CHARS).toBe(4)
    expect(TRADER_ADDR_END_CHARS).toBe(6)
    expect(shortenTraderAddress(addr)).toBe('terr…vhpax3')
    expect(shortenTraderAddress(addr)).toBe(shortenAddress(addr, 4, 6))
    expect(shortenTraderAddress(addr)).not.toBe(shortenAddress(addr, 10, 6))
    expect(shortenTraderAddress(addr)).not.toBe(addr)
  })

  it('returns short strings unchanged and empty without throwing', () => {
    expect(shortenTraderAddress('')).toBe('')
    expect(shortenTraderAddress('terr')).toBe('terr')
    expect(shortenTraderAddress('terra1ab')).toBe('terra1ab')
  })
})

describe('getTokenDisplaySymbol', () => {
  it('returns empty string for empty input', () => {
    expect(getTokenDisplaySymbol('')).toBe('')
  })

  it('returns registry symbol for known denom', () => {
    expect(getTokenDisplaySymbol('uluna')).toBe('LUNC')
    expect(getTokenDisplaySymbol('ULUNA')).toBe('LUNC')
    expect(getTokenDisplaySymbol('uusd')).toBe('USTC')
    expect(getTokenDisplaySymbol('UUSD')).toBe('USTC')
  })

  it('returns registry wrap product symbols (#507 / #630)', () => {
    expect(getTokenDisplaySymbol('terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg')).toBe('cLUNC')
    expect(getTokenDisplaySymbol('terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch')).toBe('cUSTC')
  })

  it('leaves unknown natives as the raw denom', () => {
    expect(getTokenDisplaySymbol('ufoo')).toBe('ufoo')
    expect(getTokenDisplaySymbol('ibc/ABC')).toBe('ibc/ABC')
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

describe('formatPoolAssetFieldLabel (GitLab #661)', () => {
  it('uses Name (SYMBOL) when name is distinct and short', () => {
    expect(formatPoolAssetFieldLabel({ name: 'Terra Luna Classic', symbol: 'LUNC' })).toBe('Terra Luna Classic (LUNC)')
    expect(formatPoolAssetFieldLabel({ name: 'Wrapped Luna Classic', symbol: 'cLUNC' })).toBe(
      'Wrapped Luna Classic (cLUNC)'
    )
  })

  it('collapses to symbol when name is missing or equals symbol', () => {
    expect(formatPoolAssetFieldLabel({ name: undefined, symbol: 'UST1' })).toBe('UST1')
    expect(formatPoolAssetFieldLabel({ name: 'UST1', symbol: 'UST1' })).toBe('UST1')
    expect(formatPoolAssetFieldLabel({ name: 'ust1', symbol: 'UST1' })).toBe('UST1')
    expect(formatPoolAssetFieldLabel({ name: '  ', symbol: 'GEMX' })).toBe('GEMX')
  })

  it('rejects bank denoms, HTML, and long indexer names', () => {
    expect(usablePoolAssetName('uluna', 'cLUNC')).toBe(false)
    expect(usablePoolAssetName('uusd', 'USTC')).toBe(false)
    expect(usablePoolAssetName('<img onerror=alert(1)>', 'GEMX')).toBe(false)
    expect(usablePoolAssetName('javascript:alert(1)', 'GEMX')).toBe(false)
    expect(usablePoolAssetName('one two three four five six', 'GEMX')).toBe(false)
    expect(formatPoolAssetFieldLabel({ name: 'uluna', symbol: 'cLUNC' })).toBe('cLUNC')
    expect(formatPoolAssetFieldLabel({ name: '<b>hack</b>', symbol: 'GEMX' })).toBe('GEMX')
  })

  it('builds aria labels from the product ticker', () => {
    expect(poolProvideAmountAriaLabel('LUNC')).toBe('LUNC amount')
    expect(poolProvideAmountAriaLabel('cLUNC')).toBe('cLUNC amount')
    expect(poolProvideAmountAriaLabel('UST1')).toBe('UST1 amount')
    expect(poolProvideAmountAriaLabel('')).toBe('amount')
  })
})
