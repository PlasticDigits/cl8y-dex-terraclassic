import { describe, expect, it } from 'vitest'
import {
  maxSpreadToSlippagePercent,
  slippagePercentToMaxSpread,
  spreadMatchesSlippagePercent,
} from './slippageConstraint'
describe('exact signed spread constraints #1452', () => {
  it.each([
    [0.14, '0.0014'],
    [0.29, '0.0029'],
    [0.01, '0.0001'],
    [0.5, '0.005'],
    [5, '0.05'],
    [50, '0.5'],
  ] as const)('maps %s%% exactly to %s', (percent, spread) => {
    expect(slippagePercentToMaxSpread(percent)).toBe(spread)
    expect(spreadMatchesSlippagePercent(spread, percent)).toBe(true)
    expect(maxSpreadToSlippagePercent(spread)).toBe(percent)
  })
  it.each([0, -1, 51, Infinity, NaN])('rejects invalid protection %s', (percent) => {
    expect(() => slippagePercentToMaxSpread(percent)).toThrow('Invalid slippage')
    expect(spreadMatchesSlippagePercent('0', percent)).toBe(false)
  })
  it('rejects another constraint and preserves bounded rational equivalence', () => {
    expect(spreadMatchesSlippagePercent('0.001400000000000000', 0.14)).toBe(true)
    expect(spreadMatchesSlippagePercent('0.0015', 0.14)).toBe(false)
    expect(spreadMatchesSlippagePercent('0.5', 50)).toBe(true)
    expect(spreadMatchesSlippagePercent('0.500000000000000001', 50)).toBe(false)
    expect(spreadMatchesSlippagePercent('0.000000000000000000001', 0.01)).toBe(false)
  })
})
