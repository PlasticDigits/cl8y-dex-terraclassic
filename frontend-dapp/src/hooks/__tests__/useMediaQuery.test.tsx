import { act, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { useMediaQuery } from '@/hooks/useMediaQuery'

function stubMatchMedia(matches: boolean) {
  const listeners = new Map<(e: MediaQueryListEvent) => void, (e: MediaQueryListEvent) => void>()
  const mql = {
    matches,
    media: '',
    onchange: null as null | ((e: MediaQueryListEvent) => void),
    addEventListener: vi.fn((type: string, cb: (e: MediaQueryListEvent) => void) => {
      if (type === 'change') listeners.set(cb, cb)
    }),
    removeEventListener: vi.fn((type: string, cb: (e: MediaQueryListEvent) => void) => {
      if (type === 'change') listeners.delete(cb)
    }),
    dispatchEvent: vi.fn(),
  } as unknown as MediaQueryList

  vi.stubGlobal(
    'matchMedia',
    vi.fn(() => mql)
  )

  return {
    setMatches(next: boolean) {
      Object.assign(mql, { matches: next })
      const ev = { matches: next } as MediaQueryListEvent
      for (const cb of listeners.keys()) {
        cb(ev)
      }
    },
  }
}

describe('useMediaQuery', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('reflects initial matchMedia result', () => {
    stubMatchMedia(true)
    const { result } = renderHook(() => useMediaQuery('(min-width: 1024px)'))
    expect(result.current).toBe(true)
  })

  it('updates when the query changes', () => {
    const { setMatches } = stubMatchMedia(false)
    const { result } = renderHook(() => useMediaQuery('(min-width: 1024px)'))
    expect(result.current).toBe(false)
    act(() => setMatches(true))
    expect(result.current).toBe(true)
  })
})
