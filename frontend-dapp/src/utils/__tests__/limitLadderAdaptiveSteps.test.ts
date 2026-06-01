import { describe, expect, it } from 'vitest'

import {
  computeAdaptiveMaxAdjustSteps,
  LIMIT_LADDER_ADAPTIVE_STEPS_FLOOR,
  LIMIT_LADDER_ADAPTIVE_STEPS_HARD_CAP,
} from '../limitLadderAdaptiveSteps'

describe('computeAdaptiveMaxAdjustSteps (GitLab #268)', () => {
  it('thin book stays at floor', () => {
    expect(computeAdaptiveMaxAdjustSteps({ localDepth: 0, headToBoundaryDistance: 0, rungCount: 5 })).toBe(
      LIMIT_LADDER_ADAPTIVE_STEPS_FLOOR
    )
  })

  it('deep book scales up but respects hard cap', () => {
    const steps = computeAdaptiveMaxAdjustSteps({
      localDepth: 200,
      headToBoundaryDistance: 150,
      rungCount: 20,
    })
    expect(steps).toBeGreaterThan(LIMIT_LADDER_ADAPTIVE_STEPS_FLOOR)
    expect(steps).toBeLessThanOrEqual(LIMIT_LADDER_ADAPTIVE_STEPS_HARD_CAP)
  })
})
