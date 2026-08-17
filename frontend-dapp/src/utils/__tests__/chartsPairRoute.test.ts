import { describe, expect, it } from 'vitest'
import { chartsPairHref, getInvalidChartsPairRouteParam, isChartsPairRouteParam } from '@/utils/chartsPairRoute'

const VALID = 'terra1pair0000000000000000000000000000000001'

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
