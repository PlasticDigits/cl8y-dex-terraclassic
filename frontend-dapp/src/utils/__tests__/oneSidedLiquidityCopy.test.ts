import { describe, expect, it } from 'vitest'
import * as copy from '../oneSidedLiquidityCopy'
import { formatHumanMinSwapLine, oneSidedAddPreSignAmountLines } from '../oneSidedLiquidityCopy'

describe('oneSidedLiquidityCopy (GitLab #559 T-Z12 AC7 / #660 T20)', () => {
  it('T-Z12 pre-sign min-swap is human for a 6-dec token — not raw 500571', () => {
    const line = formatHumanMinSwapLine('500571', 6)
    expect(line).toMatch(/^min swap /)
    expect(line).not.toBe('min swap 500571')
    expect(line).toMatch(/0\.5/)
    const lines = oneSidedAddPreSignAmountLines('200', '500571', 6)
    expect(lines[0]).toBe('200 in')
    expect(lines[1]).toBe(line)
    expect(lines.join(' ')).not.toMatch(/200 in min swap 500571$/)
  })

  it('T20: zap labels contain Zap; no leftover Advanced pool IA', () => {
    expect(copy.ONE_SIDED_ADD_TITLE).toMatch(/zap/i)
    expect(copy.ONE_SIDED_WITHDRAW_TITLE).toMatch(/zap/i)
    expect(copy.POOL_MANAGE_PROVIDE_LABEL).toBe('Provide Liquidity')
    expect(copy.POOL_MANAGE_WITHDRAW_LABEL).toBe('Withdraw Liquidity')
    const blob = Object.values(copy)
      .filter((v): v is string => typeof v === 'string')
      .join('\n')
    expect(blob).not.toMatch(/Advanced/i)
    expect(blob).not.toMatch(/Use Advanced/i)
  })
})
