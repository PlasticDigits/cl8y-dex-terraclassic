import { describe, expect, it } from 'vitest'
import { isCoarseNarrowViewport } from '../coarseNarrowViewport'

describe('isCoarseNarrowViewport (GitLab #632)', () => {
  it('is true only for coarse pointer and width ≤767', () => {
    expect(
      isCoarseNarrowViewport({
        innerWidth: 390,
        matchMedia: (q) => ({ matches: q.includes('pointer: coarse') }),
      })
    ).toBe(true)
    expect(
      isCoarseNarrowViewport({
        innerWidth: 390,
        matchMedia: () => ({ matches: false }),
      })
    ).toBe(false)
    expect(
      isCoarseNarrowViewport({
        innerWidth: 1440,
        matchMedia: (q) => ({ matches: q.includes('pointer: coarse') }),
      })
    ).toBe(false)
  })
})
