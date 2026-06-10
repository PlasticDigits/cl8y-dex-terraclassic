import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useDebouncedValue } from '../useDebouncedValue'

describe('useDebouncedValue', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('updates after the delay when value changes', () => {
    const { result, rerender } = renderHook(({ value, delay }) => useDebouncedValue(value, delay), {
      initialProps: { value: '1', delay: 350 },
    })
    expect(result.current).toBe('1')

    rerender({ value: '100', delay: 350 })
    expect(result.current).toBe('1')

    act(() => {
      vi.advanceTimersByTime(349)
    })
    expect(result.current).toBe('1')

    act(() => {
      vi.advanceTimersByTime(1)
    })
    expect(result.current).toBe('100')
  })

  it('coalesces rapid keystrokes into one settled value', () => {
    const { result, rerender } = renderHook(({ value }) => useDebouncedValue(value, 350), {
      initialProps: { value: '' },
    })

    rerender({ value: '1' })
    act(() => vi.advanceTimersByTime(100))
    rerender({ value: '10' })
    act(() => vi.advanceTimersByTime(100))
    rerender({ value: '100' })
    act(() => vi.advanceTimersByTime(350))

    expect(result.current).toBe('100')
  })
})
