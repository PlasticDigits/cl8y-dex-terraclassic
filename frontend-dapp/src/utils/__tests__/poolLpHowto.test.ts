import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import {
  POOL_LP_HOWTO_HINT_DISMISSED_KEY,
  POOL_LP_HOWTO_SECTION_DISMISSED_KEY,
  isPoolLpHowtoSectionHidden,
  readPoolLpHowtoHintDismissed,
  writePoolLpHowtoHintDismissed,
  writePoolLpHowtoSectionDismissed,
} from '../poolLpHowto'

describe('poolLpHowto storage (#531 / #547)', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  afterEach(() => {
    window.localStorage.clear()
  })

  it('defaults to not dismissed', () => {
    expect(readPoolLpHowtoHintDismissed()).toBe(false)
    expect(isPoolLpHowtoSectionHidden()).toBe(false)
  })

  it('persists hint dismiss in localStorage as 1/0', () => {
    writePoolLpHowtoHintDismissed(true)
    expect(window.localStorage.getItem(POOL_LP_HOWTO_HINT_DISMISSED_KEY)).toBe('1')
    expect(readPoolLpHowtoHintDismissed()).toBe(true)
  })

  it('section dismiss uses string flags and also dismisses the hint (A6)', () => {
    writePoolLpHowtoSectionDismissed(true)
    expect(window.localStorage.getItem(POOL_LP_HOWTO_SECTION_DISMISSED_KEY)).toBe('1')
    expect(window.localStorage.getItem(POOL_LP_HOWTO_HINT_DISMISSED_KEY)).toBe('1')
    expect(isPoolLpHowtoSectionHidden()).toBe(true)
  })
})
