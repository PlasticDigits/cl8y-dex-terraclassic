import type { CSSProperties } from 'react'

export const PORTAL_LISTBOX_VIEWPORT_PAD = 8
export const PORTAL_LISTBOX_MIN_MENU_HEIGHT = 120

export type PortalListboxViewport = {
  width: number
  height: number
  /** Fixed chrome + IME / in-app reserve below the layout viewport (GitLab #632). */
  bottomInset?: number
  /** Visible top band reserved for sticky chrome or visual-viewport offset. */
  topInset?: number
  /** Visual-viewport offset from the left of the layout viewport. */
  offsetLeft?: number
}

export type PortalListboxPositionArgs = {
  anchor: DOMRectReadOnly
  viewport: PortalListboxViewport
  preferredMaxHeight?: number
  gap?: number
}

/**
 * Fixed coordinates for a portaled listbox anchored to a trigger rect.
 * Pure function — safe to unit test and call during render when the anchor ref is set.
 *
 * `position: fixed` is layout-viewport-relative. Pass visualViewport and chrome
 * occlusion as `topInset` / `bottomInset` (see `readPortalListboxViewport`) so the menu stays
 * inside the visible band and above DEX / in-app chrome (GitLab #632).
 */
export function computePortalListboxStyle({
  anchor,
  viewport,
  preferredMaxHeight = 280,
  gap = 8,
}: PortalListboxPositionArgs): CSSProperties {
  const vw = Math.max(1, Number.isFinite(viewport.width) ? viewport.width : 1)
  const vh = Math.max(1, Number.isFinite(viewport.height) ? viewport.height : 1)
  const bottomBar = Math.max(0, viewport.bottomInset ?? 0)
  const topBar = Math.max(0, viewport.topInset ?? 0)
  const originLeft = Math.max(0, viewport.offsetLeft ?? 0)

  const width = Math.min(anchor.width, vw - 2 * PORTAL_LISTBOX_VIEWPORT_PAD)
  let left = anchor.left + (anchor.width - width) / 2
  left = Math.min(
    Math.max(originLeft + PORTAL_LISTBOX_VIEWPORT_PAD, left),
    originLeft + vw - PORTAL_LISTBOX_VIEWPORT_PAD - width
  )

  const visibleTop = topBar + PORTAL_LISTBOX_VIEWPORT_PAD
  const visibleBottom = Math.max(visibleTop, vh - bottomBar - PORTAL_LISTBOX_VIEWPORT_PAD)
  // A scroll can carry an open picker trigger behind fixed chrome or out of
  // view. Clamp the attachment point as well as the height so its portal stays
  // usable until the user selects an option or dismisses it (#1316).
  const belowTop = Math.min(visibleBottom, Math.max(visibleTop, anchor.bottom + gap))
  const aboveBottom = Math.min(visibleBottom, Math.max(visibleTop, anchor.top - gap))
  const availBelow = visibleBottom - belowTop
  const availAbove = aboveBottom - visibleTop

  const preferBelow = availBelow >= PORTAL_LISTBOX_MIN_MENU_HEIGHT || availBelow >= availAbove

  if (preferBelow) {
    return {
      position: 'fixed',
      top: belowTop,
      left,
      width,
      maxHeight: Math.min(preferredMaxHeight, availBelow),
      bottom: 'auto',
    }
  }

  return {
    position: 'fixed',
    top: 'auto',
    left,
    width,
    maxHeight: Math.min(preferredMaxHeight, availAbove),
    bottom: vh - aboveBottom,
  }
}

/** Layout-viewport Y of the reserved band (listbox must stay strictly above this). */
export function portalListboxReservedTop(viewport: PortalListboxViewport): number {
  const vh = Math.max(1, Number.isFinite(viewport.height) ? viewport.height : 1)
  const bottomBar = Math.max(0, viewport.bottomInset ?? 0)
  return vh - bottomBar
}
