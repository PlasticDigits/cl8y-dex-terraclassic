import { describe, expect, it } from 'vitest'
import { parseHubPriceTicker } from '../hubPriceTicker'

describe('parseHubPriceTicker', () => {
  it('allowlists custc / ust1 / ustr', () => {
    expect(parseHubPriceTicker('ustr')).toBe('ustr')
    expect(parseHubPriceTicker('UST1')).toBe('ust1')
    expect(parseHubPriceTicker('cUSTC')).toBe('custc')
  })

  it('rejects CEX tickers and injection', () => {
    expect(parseHubPriceTicker('ustc')).toBeNull()
    expect(parseHubPriceTicker('vfdusd')).toBeNull()
    expect(parseHubPriceTicker('../ustr')).toBeNull()
    expect(parseHubPriceTicker('javascript:alert(1)')).toBeNull()
    expect(parseHubPriceTicker('ustr_')).toBeNull()
  })
})
