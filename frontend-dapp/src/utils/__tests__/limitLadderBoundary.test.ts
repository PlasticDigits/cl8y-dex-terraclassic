import { describe, expect, it } from 'vitest'

import { ladderBoundaryRungIndex } from '../limitLadderBoundary'

describe('ladderBoundaryRungIndex', () => {
  it('bid ascending start→end: boundary is last rung (head-most price)', () => {
    expect(ladderBoundaryRungIndex('bid', '0.95', '1.05', 5)).toBe(4)
  })

  it('bid descending start→end: boundary is first rung', () => {
    expect(ladderBoundaryRungIndex('bid', '1.05', '0.95', 5)).toBe(0)
  })

  it('ask ascending start→end: boundary is first rung (lowest price)', () => {
    expect(ladderBoundaryRungIndex('ask', '0.95', '1.05', 5)).toBe(0)
  })

  it('ask descending start→end: boundary is last rung', () => {
    expect(ladderBoundaryRungIndex('ask', '1.05', '0.95', 5)).toBe(4)
  })
})
