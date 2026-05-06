import { describe, expect, it, vi } from 'vitest'

import { warnIndexerPlacementPollFailed } from './warnIndexerPlacementPollFailed'

describe('warnIndexerPlacementPollFailed', () => {
  it('emits a tagged console warning', () => {
    const spy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const err = new Error('test')
    warnIndexerPlacementPollFailed(err)
    expect(spy).toHaveBeenCalledWith('[limit-place] indexer poll failed:', err)
    spy.mockRestore()
  })
})
