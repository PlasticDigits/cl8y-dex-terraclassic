import { describe, expect, it, vi, afterEach } from 'vitest'
import { getMobileBottomNavInsetPx } from '@/lib/mobileBottomNav'

describe('getMobileBottomNavInsetPx', () => {
  afterEach(() => {
    document.body.innerHTML = ''
    vi.restoreAllMocks()
  })

  it('returns 0 when the mobile nav is absent', () => {
    expect(getMobileBottomNavInsetPx()).toBe(0)
  })

  it('returns 0 when the nav is display:none', () => {
    const nav = document.createElement('nav')
    nav.className = 'app-mobile-nav-shell'
    nav.style.display = 'none'
    document.body.appendChild(nav)
    expect(getMobileBottomNavInsetPx()).toBe(0)
  })

  it('returns ceil of bounding height when the nav is visible', () => {
    const nav = document.createElement('nav')
    nav.className = 'app-mobile-nav-shell'
    document.body.appendChild(nav)
    vi.spyOn(nav, 'getBoundingClientRect').mockReturnValue({
      height: 88.2,
      width: 400,
      x: 0,
      y: 0,
      top: 0,
      left: 0,
      right: 400,
      bottom: 88.2,
      toJSON: () => ({}),
    } as DOMRect)
    expect(getMobileBottomNavInsetPx()).toBe(89)
  })
})
