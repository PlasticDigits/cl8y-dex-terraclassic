import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  SOUNDS_ENABLED_STORAGE_KEY,
  readSoundsEnabled,
  resetSoundsEnabledCacheForTests,
  writeSoundsEnabled,
} from './soundPreferences'

describe('soundPreferences', () => {
  beforeEach(() => {
    window.localStorage.clear()
    resetSoundsEnabledCacheForTests()
  })

  afterEach(() => {
    window.localStorage.clear()
    resetSoundsEnabledCacheForTests()
  })

  it('defaults to enabled when the key is missing', () => {
    expect(readSoundsEnabled()).toBe(true)
  })

  it('reads stored off and on values', () => {
    window.localStorage.setItem(SOUNDS_ENABLED_STORAGE_KEY, '0')
    expect(readSoundsEnabled()).toBe(false)
    resetSoundsEnabledCacheForTests()
    window.localStorage.setItem(SOUNDS_ENABLED_STORAGE_KEY, '1')
    expect(readSoundsEnabled()).toBe(true)
  })

  it('treats garbage storage as default ON', () => {
    window.localStorage.setItem(SOUNDS_ENABLED_STORAGE_KEY, 'maybe')
    expect(readSoundsEnabled()).toBe(true)
  })

  it('round-trips writeSoundsEnabled via localStorage', () => {
    writeSoundsEnabled(false)
    expect(window.localStorage.getItem(SOUNDS_ENABLED_STORAGE_KEY)).toBe('0')
    expect(readSoundsEnabled()).toBe(false)

    writeSoundsEnabled(true)
    expect(window.localStorage.getItem(SOUNDS_ENABLED_STORAGE_KEY)).toBe('1')
    expect(readSoundsEnabled()).toBe(true)
  })

  it('keeps the session value when localStorage.setItem throws', () => {
    const setItem = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('quota')
    })
    writeSoundsEnabled(false)
    expect(readSoundsEnabled()).toBe(false)
    setItem.mockRestore()
  })
})
