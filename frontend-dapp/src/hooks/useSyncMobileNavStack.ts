import { useLayoutEffect, type RefObject } from 'react'

/**
 * Writes `--app-mobile-nav-stack` on `shellRef` from the live height of the fixed
 * mobile bottom bar (padding, safe-area, wrapping). Lets overlays (e.g. More sheet)
 * clear the bar without hard-coding pixel estimates.
 */
export function useSyncMobileNavStack(
  shellRef: RefObject<HTMLElement | null>,
  mobileNavRef: RefObject<HTMLElement | null>
): void {
  useLayoutEffect(() => {
    const shell = shellRef.current
    const nav = mobileNavRef.current
    if (!shell || !nav) return

    const sync = () => {
      const h = nav.getBoundingClientRect().height
      if (h < 1) {
        shell.style.removeProperty('--app-mobile-nav-stack')
        return
      }
      shell.style.setProperty('--app-mobile-nav-stack', `${Math.ceil(h)}px`)
    }

    sync()
    const ro = new ResizeObserver(sync)
    ro.observe(nav)
    window.addEventListener('resize', sync)
    return () => {
      ro.disconnect()
      window.removeEventListener('resize', sync)
    }
  }, [shellRef, mobileNavRef])
}
