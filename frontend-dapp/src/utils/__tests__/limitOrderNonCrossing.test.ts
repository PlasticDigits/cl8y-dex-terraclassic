import { describe, it, expect } from 'vitest'
import {
  comparePositiveDecimalStrings,
  limitAskCrossesBestBid,
  limitBidCrossesBestAsk,
  describeLimitCrossingBlocker,
} from '@/utils/limitOrderNonCrossing'

describe('limitOrderNonCrossing', () => {
  it('compares decimal strings', () => {
    expect(comparePositiveDecimalStrings('1', '2')).toBe('lt')
    expect(comparePositiveDecimalStrings('2', '1')).toBe('gt')
    expect(comparePositiveDecimalStrings('1.0', '1')).toBe('eq')
    expect(comparePositiveDecimalStrings('01.10', '1.1')).toBe('eq')
    expect(comparePositiveDecimalStrings('10', '9.99')).toBe('gt')
  })

  it('detects bid crossing best ask', () => {
    expect(limitBidCrossesBestAsk('1.5', null)).toBe(false)
    expect(limitBidCrossesBestAsk('1.5', '2')).toBe(false)
    expect(limitBidCrossesBestAsk('2', '2')).toBe(true)
    expect(limitBidCrossesBestAsk('2.1', '2')).toBe(true)
  })

  it('detects ask crossing best bid', () => {
    expect(limitAskCrossesBestBid('1.5', null)).toBe(false)
    expect(limitAskCrossesBestBid('2', '1')).toBe(false)
    expect(limitAskCrossesBestBid('1', '1')).toBe(true)
    expect(limitAskCrossesBestBid('0.9', '1')).toBe(true)
  })

  it('describeLimitCrossingBlocker', () => {
    expect(describeLimitCrossingBlocker('bid', '2', '1', '1.5')).toContain('best ask')
    expect(describeLimitCrossingBlocker('ask', '0.5', '1', '2')).toContain('best bid')
    expect(describeLimitCrossingBlocker('bid', '1', '2', '3')).toBeNull()
  })
})
