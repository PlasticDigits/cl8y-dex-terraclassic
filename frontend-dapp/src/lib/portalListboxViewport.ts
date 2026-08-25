import type { PortalListboxViewport } from '@/components/ui/portalListboxPosition'
import { getMobileBottomNavInsetPx } from '@/lib/mobileBottomNav'
import { isCoarseNarrowViewport } from '@/lib/coarseNarrowViewport'
import { detectWalletInAppBrowser } from '@/utils/detectWalletInAppBrowser'

/** Extra reserve for Keplr / Station / Cosmostation in-app URL chrome (GitLab #632). */
export const PORTAL_LISTBOX_IN_APP_CHROME_RESERVE_PX = 56

/** Minimum finger gap between the listbox bottom and the reserved band (GitLab #632). */
export const PORTAL_LISTBOX_FINGER_GAP_PX = 44

export type VisualViewportLike = {
  width: number
  height: number
  offsetTop: number
  offsetLeft: number
}

export type ReadPortalListboxViewportArgs = {
  innerWidth: number
  innerHeight: number
  visualViewport?: VisualViewportLike | null
  tabBarInset: number
  inAppChrome: boolean
  coarseNarrow: boolean
}

function finiteNonNegative(n: unknown, fallback: number): number {
  return typeof n === 'number' && Number.isFinite(n) && n >= 0 ? n : fallback
}

/**
 * Layout-viewport coords for `position: fixed`, with visualViewport occlusion
 * and optional in-app chrome folded into insets (GitLab #632).
 *
 * Spoofed / extreme visualViewport values clamp to a usable box — never throw
 * and never treat the reserved band as tappable menu space.
 */
export function readPortalListboxViewport(args: ReadPortalListboxViewportArgs): PortalListboxViewport {
  const innerW = Math.max(1, finiteNonNegative(args.innerWidth, 1))
  const innerH = Math.max(1, finiteNonNegative(args.innerHeight, 1))
  const vv = args.visualViewport ?? null

  const vvW = vv ? Math.min(innerW, Math.max(1, finiteNonNegative(vv.width, innerW))) : innerW
  const vvH = vv ? Math.min(innerH, Math.max(1, finiteNonNegative(vv.height, innerH))) : innerH
  const offsetTop = vv ? Math.min(innerH, finiteNonNegative(vv.offsetTop, 0)) : 0
  const offsetLeft = vv ? Math.min(innerW, finiteNonNegative(vv.offsetLeft, 0)) : 0
  const visualBottom = Math.min(innerH, offsetTop + vvH)
  const bottomOccluded = Math.max(0, innerH - visualBottom)

  const tab = Math.max(0, finiteNonNegative(args.tabBarInset, 0))
  const inApp = args.inAppChrome && args.coarseNarrow ? PORTAL_LISTBOX_IN_APP_CHROME_RESERVE_PX : 0
  const needFinger = args.coarseNarrow || inApp > 0 || bottomOccluded > 0
  const finger = needFinger ? PORTAL_LISTBOX_FINGER_GAP_PX : 0

  return {
    width: vvW,
    height: innerH,
    offsetLeft,
    topInset: offsetTop,
    bottomInset: tab + bottomOccluded + inApp + finger,
  }
}

/** Live viewport snapshot for `usePortalListbox` (first-frame sync + listeners). */
export function getPortalListboxViewport(): PortalListboxViewport {
  if (typeof window === 'undefined') {
    return { width: 1, height: 1, bottomInset: 0 }
  }
  const vv = window.visualViewport
  return readPortalListboxViewport({
    innerWidth: window.innerWidth,
    innerHeight: window.innerHeight,
    visualViewport: vv
      ? {
          width: vv.width,
          height: vv.height,
          offsetTop: vv.offsetTop,
          offsetLeft: vv.offsetLeft,
        }
      : null,
    tabBarInset: getMobileBottomNavInsetPx(),
    inAppChrome: detectWalletInAppBrowser().isInAppBrowser,
    coarseNarrow: isCoarseNarrowViewport(),
  })
}
