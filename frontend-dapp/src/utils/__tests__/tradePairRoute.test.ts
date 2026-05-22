import { describe, expect, it } from 'vitest'
import { getInvalidTradePairRouteParam, isTradePairRouteParam } from '@/utils/tradePairRoute'

const VALID = 'terra1pair0000000000000000000000000000000001'

describe('tradePairRoute', () => {
  it('accepts valid terra1 pair addresses', () => {
    expect(isTradePairRouteParam(VALID)).toBe(true)
    expect(getInvalidTradePairRouteParam(VALID)).toBeNull()
  })

  it('rejects non-terra1 garbage deep links', () => {
    expect(isTradePairRouteParam('lilwayne babyyy')).toBe(false)
    expect(getInvalidTradePairRouteParam('lilwayne babyyy')).toBe('lilwayne babyyy')
  })

  it('rejects terra1 prefix without full address', () => {
    expect(isTradePairRouteParam('terra1')).toBe(false)
    expect(getInvalidTradePairRouteParam('terra1')).toBe('terra1')
  })

  it('returns null when route param is absent', () => {
    expect(isTradePairRouteParam(undefined)).toBe(false)
    expect(getInvalidTradePairRouteParam(undefined)).toBeNull()
  })
})
