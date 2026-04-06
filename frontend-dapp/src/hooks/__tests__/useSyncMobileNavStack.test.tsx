import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { render } from '@testing-library/react'
import { useRef } from 'react'
import { useSyncMobileNavStack } from '@/hooks/useSyncMobileNavStack'

function Probe() {
  const shellRef = useRef<HTMLDivElement>(null)
  const navRef = useRef<HTMLElement>(null)
  useSyncMobileNavStack(shellRef, navRef)

  return (
    <div ref={shellRef} data-testid="shell">
      <nav ref={navRef} data-testid="nav" />
    </div>
  )
}

describe('useSyncMobileNavStack', () => {
  const RO = globalThis.ResizeObserver

  beforeEach(() => {
    globalThis.ResizeObserver = vi.fn(function ResizeObserver(this: ResizeObserver, cb: ResizeObserverCallback) {
      this.observe = vi.fn()
      this.unobserve = vi.fn()
      this.disconnect = vi.fn()
      queueMicrotask(() => {
        cb([], this)
      })
    }) as unknown as typeof ResizeObserver
  })

  afterEach(() => {
    globalThis.ResizeObserver = RO
    vi.restoreAllMocks()
  })

  it('sets --app-mobile-nav-stack from the measured nav height', () => {
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({
      height: 96,
      width: 400,
      x: 0,
      y: 0,
      top: 0,
      left: 0,
      right: 400,
      bottom: 96,
      toJSON: () => ({}),
    } as DOMRect)

    const { getByTestId } = render(<Probe />)
    const shell = getByTestId('shell')
    expect(shell.style.getPropertyValue('--app-mobile-nav-stack').trim()).toBe('96px')
  })

  it('clears the custom property when the nav reports zero height', () => {
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({
      height: 0,
      width: 0,
      x: 0,
      y: 0,
      top: 0,
      left: 0,
      right: 0,
      bottom: 0,
      toJSON: () => ({}),
    } as DOMRect)

    const { getByTestId } = render(<Probe />)
    const shell = getByTestId('shell')
    expect(shell.style.getPropertyValue('--app-mobile-nav-stack')).toBe('')
  })
})
