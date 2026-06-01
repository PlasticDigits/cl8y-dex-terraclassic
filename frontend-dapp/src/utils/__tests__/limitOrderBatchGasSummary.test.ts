import { describe, expect, it } from 'vitest'

import { formatLimitLadderPlacementSummary } from '../limitOrderBatchGasSummary'

describe('formatLimitLadderPlacementSummary (GitLab #268)', () => {
  it('includes path and expected rungs', () => {
    const line = formatLimitLadderPlacementSummary(5, 48, {
      path: 'deep_batch',
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
    })
    expect(line).toContain('hinted batch')
    expect(line).toContain('4/5')
    expect(line).toContain('48')
  })
})
