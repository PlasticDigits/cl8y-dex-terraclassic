import { describe, expect, it } from 'vitest'
import { parseHubPriceTicker } from '../hubPriceTicker'

describe('parseHubPriceTicker', () => {
  it('allowlists custc / lunc / ust1 / ustr', () => {
    expect(parseHubPriceTicker('ustr')).toBe('ustr')
    expect(parseHubPriceTicker('UST1')).toBe('ust1')
    expect(parseHubPriceTicker('cUSTC')).toBe('custc')
    expect(parseHubPriceTicker('lunc')).toBe('lunc')
    expect(parseHubPriceTicker('LUNC')).toBe('lunc')
  })

  it('rejects CEX aliases, clunc path, and injection', () => {
    expect(parseHubPriceTicker('ustc')).toBeNull()
    expect(parseHubPriceTicker('vfdusd')).toBeNull()
    expect(parseHubPriceTicker('clunc')).toBeNull()
    expect(parseHubPriceTicker('../ustr')).toBeNull()
    expect(parseHubPriceTicker('../lunc')).toBeNull()
    expect(parseHubPriceTicker('javascript:alert(1)')).toBeNull()
    expect(parseHubPriceTicker('ustr_')).toBeNull()
    expect(parseHubPriceTicker('lunc_')).toBeNull()
    expect(parseHubPriceTicker('lunc\u200b')).toBeNull()
  })
})
