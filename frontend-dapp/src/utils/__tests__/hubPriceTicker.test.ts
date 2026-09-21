import { describe, expect, it } from 'vitest'
import { HUB_PRICE_TICKER_LABEL, parseHubPriceTicker } from '../hubPriceTicker'

describe('parseHubPriceTicker', () => {
  it('prints wrap labels cUSTC / cLUNC without remapping ids (#1240)', () => {
    expect(HUB_PRICE_TICKER_LABEL.custc).toBe('cUSTC')
    expect(HUB_PRICE_TICKER_LABEL.lunc).toBe('cLUNC')
    expect(HUB_PRICE_TICKER_LABEL.ust1).toBe('UST1')
    expect(HUB_PRICE_TICKER_LABEL.ustr).toBe('USTR')
  })

  it('allowlists custc / lunc / ust1 / ustr', () => {
    expect(parseHubPriceTicker('ustr')).toBe('ustr')
    expect(parseHubPriceTicker('UST1')).toBe('ust1')
    expect(parseHubPriceTicker('cUSTC')).toBe('custc')
    expect(parseHubPriceTicker('lunc')).toBe('lunc')
    expect(parseHubPriceTicker('LUNC')).toBe('lunc')
    expect(parseHubPriceTicker('cLUNC')).toBeNull()
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
