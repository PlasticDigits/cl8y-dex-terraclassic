import { describe, it, expect } from 'vitest'
import { mapWithConcurrency } from '../mapWithConcurrency'

describe('mapWithConcurrency', () => {
  it('returns empty for no items', async () => {
    expect(await mapWithConcurrency([], 3, async () => 1)).toEqual([])
  })

  it('preserves order with concurrency cap', async () => {
    const out = await mapWithConcurrency([1, 2, 3, 4], 2, async (n) => n * 2)
    expect(out).toEqual([2, 4, 6, 8])
  })
})
