import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import {
  SWAP_SETTINGS_ADVANCED_OPEN_KEY,
  readSwapSettingsAdvancedOpen,
  writeSwapSettingsAdvancedOpen,
} from './swapSettingsAdvanced'

describe('swapSettingsAdvanced', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  afterEach(() => {
    window.localStorage.clear()
  })

  it('defaults to collapsed when unset', () => {
    expect(readSwapSettingsAdvancedOpen()).toBe(false)
  })

  it('persists expanded state', () => {
    writeSwapSettingsAdvancedOpen(true)
    expect(window.localStorage.getItem(SWAP_SETTINGS_ADVANCED_OPEN_KEY)).toBe('1')
    expect(readSwapSettingsAdvancedOpen()).toBe(true)
  })

  it('persists collapsed state', () => {
    writeSwapSettingsAdvancedOpen(true)
    writeSwapSettingsAdvancedOpen(false)
    expect(readSwapSettingsAdvancedOpen()).toBe(false)
  })
})
