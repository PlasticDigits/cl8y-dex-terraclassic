import { useLayoutEffect, type RefObject } from 'react'

/**
 * Writes `--app-mobile-nav-stack` on `document.documentElement` from the live height
 * of the fixed mobile bottom bar (padding, safe-area, wrapping). Inherited by
 * `.app-shell`, CSS, and JS (e.g. {@link getMobileBottomNavInsetPx}) consumers.
 */
export function useSyncMobileNavStack(mobileNavRef: RefObject<HTMLElement | null>): void {
  useLayoutEffect(() => {
    const nav = mobileNavRef.current
    if (!nav) return
    const root = document.documentElement

    const sync = () => {
      const h = nav.getBoundingClientRect().height
      if (h < 1) {
        root.style.removeProperty('--app-mobile-nav-stack')
        return
      }
      root.style.setProperty('--app-mobile-nav-stack', `${Math.ceil(h)}px`)
    }

    sync()
    const ro = new ResizeObserver(sync)
    ro.observe(nav)
    window.addEventListener('resize', sync)
    return () => {
      ro.disconnect()
      window.removeEventListener('resize', sync)
      root.style.removeProperty('--app-mobile-nav-stack')
    }
  }, [mobileNavRef])
}
