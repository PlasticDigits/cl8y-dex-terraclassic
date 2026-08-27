import { describe, expect, it } from 'vitest'
import { MAINNET_CUSTC_TOKEN_ADDRESS, MAINNET_UST1_TOKEN_ADDRESS } from '@/utils/ust1SecondaryMarket'
import {
  chartsPairHref,
  getInvalidChartsPairRouteParam,
  isChartsPairRouteParam,
  isSafeChartsPriceToken,
  matchChartsPriceParam,
  parseChartsPriceQuery,
} from '@/utils/chartsPairRoute'

const VALID = 'terra1pair0000000000000000000000000000000001'
const UST1 = { symbol: 'UST1', contractAddr: MAINNET_UST1_TOKEN_ADDRESS }
const CUSTC = { symbol: 'cUSTC', contractAddr: MAINNET_CUSTC_TOKEN_ADDRESS }
const USTR = { symbol: 'USTR', contractAddr: 'terra1ustrxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx' }

describe('chartsPairRoute (GitLab #547)', () => {
  it('accepts valid terra1 pair addresses', () => {
    expect(isChartsPairRouteParam(VALID)).toBe(true)
    expect(getInvalidChartsPairRouteParam(VALID)).toBeNull()
    expect(chartsPairHref(VALID)).toBe(`/charts/${VALID}`)
  })

  it('rejects non-bech32, javascript:, and HTML (A1)', () => {
    expect(isChartsPairRouteParam('javascript:alert(1)')).toBe(false)
    expect(isChartsPairRouteParam('<script>')).toBe(false)
    expect(isChartsPairRouteParam('https://evil.example')).toBe(false)
    expect(chartsPairHref('javascript:alert(1)')).toBeNull()
    expect(chartsPairHref('<script>')).toBeNull()
    expect(chartsPairHref('not-a-terra')).toBeNull()
    expect(getInvalidChartsPairRouteParam('not-a-terra')).toBe('not-a-terra')
    expect(getInvalidChartsPairRouteParam('<script>')).toBe('<script>')
  })

  it('rejects terra1 prefix without a full address', () => {
    expect(isChartsPairRouteParam('terra1')).toBe(false)
    expect(chartsPairHref('terra1')).toBeNull()
  })
})

describe('chartsPairRoute ?price= (GitLab #680)', () => {
  it('appends only safe price tokens', () => {
    expect(chartsPairHref(VALID, { price: 'UST1' })).toBe(`/charts/${VALID}?price=UST1`)
    expect(chartsPairHref(VALID, { price: 'cUSTC' })).toBe(`/charts/${VALID}?price=cUSTC`)
    expect(chartsPairHref(VALID, { price: '<script>' })).toBe(`/charts/${VALID}`)
    expect(chartsPairHref(VALID, { price: 'javascript:alert(1)' })).toBe(`/charts/${VALID}`)
    expect(isSafeChartsPriceToken('<script>')).toBe(false)
    expect(isSafeChartsPriceToken('javascript:alert(1)')).toBe(false)
  })

  it('parses last price key and ignores hostile / overlong values', () => {
    expect(parseChartsPriceQuery('price=UST1')).toBe('UST1')
    expect(parseChartsPriceQuery('price=ust1')).toBe('ust1')
    expect(parseChartsPriceQuery('foo=1&price=a&price=UST1')).toBe('UST1')
    expect(parseChartsPriceQuery('price=javascript:alert(1)')).toBeNull()
    expect(parseChartsPriceQuery('price=<script>')).toBeNull()
    expect(parseChartsPriceQuery('price=//evil')).toBeNull()
    expect(parseChartsPriceQuery(`price=${'x'.repeat(4000)}`)).toBeNull()
    expect(parseChartsPriceQuery('')).toBeNull()
    expect(parseChartsPriceQuery('price=')).toBeNull()
  })

  it('matches pair legs and USTC/LUNC aliases only when that wrap is on the pair', () => {
    expect(matchChartsPriceParam('UST1', UST1, CUSTC, VALID)).toBe('asset0')
    expect(matchChartsPriceParam('ust1', UST1, CUSTC, VALID)).toBe('asset0')
    expect(matchChartsPriceParam('cUSTC', UST1, CUSTC, VALID)).toBe('asset1')
    expect(matchChartsPriceParam('USTC', UST1, CUSTC, VALID)).toBe('asset1')
    expect(matchChartsPriceParam(MAINNET_CUSTC_TOKEN_ADDRESS, UST1, CUSTC, VALID)).toBe('asset1')
    expect(matchChartsPriceParam('UST1', UST1, USTR, VALID)).toBe('asset0')
    expect(matchChartsPriceParam('cUSTC', UST1, USTR, VALID)).toBeNull()
    expect(matchChartsPriceParam('USTC', UST1, USTR, VALID)).toBeNull()
    expect(matchChartsPriceParam(VALID, UST1, CUSTC, VALID)).toBeNull()
    expect(matchChartsPriceParam('javascript:alert(1)', UST1, CUSTC, VALID)).toBeNull()
    expect(matchChartsPriceParam('<script>', UST1, CUSTC, VALID)).toBeNull()
  })
})
