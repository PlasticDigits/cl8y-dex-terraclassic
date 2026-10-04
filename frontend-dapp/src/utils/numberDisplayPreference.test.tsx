import { act, render, screen } from '@testing-library/react'
import { useState } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  NUMBER_DISPLAY_MODE_KEY,
  readNumberDisplayMode,
  useNumberDisplayMode,
  writeNumberDisplayMode,
} from './numberDisplayPreference'
import { formatTokenAmount } from './formatAmount'

beforeEach(() => localStorage.clear())
afterEach(() => vi.restoreAllMocks())

describe('number display preference', () => {
  it.each([null, '', 'compact', 'FULL', 'unknown', '<img>', 'javascript:alert(1)'])(
    'defaults %s to Compact',
    (value) => {
      if (value !== null) localStorage.setItem(NUMBER_DISPLAY_MODE_KEY, value)
      expect(readNumberDisplayMode()).toBe('compact')
    }
  )

  it('persists only legal modes and reads Full after reload', () => {
    writeNumberDisplayMode('full')
    expect(readNumberDisplayMode()).toBe('full')
    expect(localStorage.getItem(NUMBER_DISPLAY_MODE_KEY)).toBe('full')
    writeNumberDisplayMode('compact')
    expect(readNumberDisplayMode()).toBe('compact')
  })

  it('falls back to Compact with unavailable or quota-limited storage', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('private mode')
    })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('quota')
    })
    expect(() => writeNumberDisplayMode('full')).not.toThrow()
    expect(readNumberDisplayMode()).toBe('compact')
  })

  it('ignores inbound number-format query keys', () => {
    window.history.replaceState({}, '', '/?format=full&compact=0&locale=de-DE')
    expect(readNumberDisplayMode()).toBe('compact')
    window.history.replaceState({}, '', '/')
  })

  it('updates all mounted subscribers and cross-tab changes without remounting', () => {
    const mounts = vi.fn()
    function Amount({ label }: { label: string }) {
      useNumberDisplayMode()
      const [draft] = useState(() => {
        mounts()
        return '1000.5'
      })
      return (
        <span data-testid={label}>
          {formatTokenAmount('1234567890000', 6)} / {draft}
        </span>
      )
    }
    render(
      <>
        <Amount label="balance" />
        <Amount label="tape" />
      </>
    )
    expect(screen.getByTestId('balance')).toHaveTextContent('1.235M / 1000.5')
    act(() => writeNumberDisplayMode('full'))
    for (const label of ['balance', 'tape'])
      expect(screen.getByTestId(label)).toHaveTextContent('1,234,567.89 / 1000.5')
    act(() => {
      localStorage.setItem(NUMBER_DISPLAY_MODE_KEY, 'compact')
      window.dispatchEvent(new StorageEvent('storage', { key: NUMBER_DISPLAY_MODE_KEY }))
    })
    expect(screen.getByTestId('tape')).toHaveTextContent('1.235M / 1000.5')
    expect(mounts).toHaveBeenCalledTimes(2)
  })
})
