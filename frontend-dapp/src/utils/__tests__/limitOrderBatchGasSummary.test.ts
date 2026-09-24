import { describe, expect, it } from 'vitest'

import { formatLimitLadderPlacementSummary } from '../limitOrderBatchGasSummary'

describe('formatLimitLadderPlacementSummary (GitLab #268)', () => {
  const plan = {
    path: 'deep_batch' as const,
    recommendedMaxSteps: 48,
    skipRisk: { score: 2, predictedPlaced: 4, predictedSkipped: 1, needsHintedBatchPath: true },
    depth: {
      windowOrderCount: 10,
      foreignOrdersBetweenRungs: 1,
      headToBoundaryDistance: 3,
      unresolvedHintCount: 0,
    },
    hints: [],
    probeDegraded: false,
    notes: [],
  }

  it('includes path and expected rungs', () => {
    const line = formatLimitLadderPlacementSummary(5, 48, plan)
    expect(line).toContain('hinted batch')
    expect(line).toContain('4/5')
    expect(line).toContain('LUNC')
  })

  it('shows a larger gas estimate when a ladder allows a deeper insertion walk', () => {
    const low = formatLimitLadderPlacementSummary(5, 16, plan)
    const high = formatLimitLadderPlacementSummary(5, 128, plan)
    expect(high).not.toBe(low)
  })
})
