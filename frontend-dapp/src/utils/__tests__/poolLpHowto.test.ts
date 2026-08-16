import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import {
  POOL_LP_HOWTO_HINT_DISMISSED_KEY,
  readPoolLpHowtoHintDismissed,
  writePoolLpHowtoHintDismissed,
} from '../poolLpHowto'

describe('poolLpHowto storage (#531)', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  afterEach(() => {
    window.localStorage.clear()
  })

  it('defaults to not dismissed', () => {
    expect(readPoolLpHowtoHintDismissed()).toBe(false)
  })

  it('persists dismiss in localStorage', () => {
    writePoolLpHowtoHintDismissed(true)
    expect(window.localStorage.getItem(POOL_LP_HOWTO_HINT_DISMISSED_KEY)).toBe('1')
    expect(readPoolLpHowtoHintDismissed()).toBe(true)
  })
})
