import { describe, expect, it } from 'vitest'
import { formatHumanMinSwapLine, oneSidedAddPreSignAmountLines } from '../oneSidedLiquidityCopy'

describe('oneSidedLiquidityCopy (GitLab #559 T-Z12 AC7)', () => {
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
})
