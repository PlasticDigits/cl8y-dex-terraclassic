import { describe, it, expect } from 'vitest'
import { limitPriceUsdHint, inverseLimitPriceHuman } from '@/utils/tradeLimitPriceDisplay'

describe('tradeLimitPriceDisplay', () => {
  it('inverseLimitPriceHuman', () => {
    expect(inverseLimitPriceHuman('2')).toBeTruthy()
    expect(inverseLimitPriceHuman('0')).toBeNull()
  })

  it('limitPriceUsdHint when quote is USTC', () => {
    const h = limitPriceUsdHint('10', 'USTC', '0.02')
    expect(h).toContain('$')
    expect(h).toContain('0.2')
  })

  it('limitPriceUsdHint skips non-stable quote', () => {
    expect(limitPriceUsdHint('10', 'BBB', '0.02')).toBeNull()
  })
})
