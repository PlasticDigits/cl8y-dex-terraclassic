import type { CSSProperties } from 'react'

export const PORTAL_LISTBOX_VIEWPORT_PAD = 8
export const PORTAL_LISTBOX_MIN_MENU_HEIGHT = 120

export type PortalListboxViewport = {
  width: number
  height: number
  /** Fixed chrome below the viewport (e.g. mobile tab bar). */
  bottomInset?: number
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
 */
export function computePortalListboxStyle({
  anchor,
  viewport,
  preferredMaxHeight = 280,
  gap = 8,
}: PortalListboxPositionArgs): CSSProperties {
  const vw = viewport.width
  const vh = viewport.height
  const bottomBar = viewport.bottomInset ?? 0

  const width = Math.min(anchor.width, vw - 2 * PORTAL_LISTBOX_VIEWPORT_PAD)
  let left = anchor.left + (anchor.width - width) / 2
  left = Math.min(Math.max(PORTAL_LISTBOX_VIEWPORT_PAD, left), vw - PORTAL_LISTBOX_VIEWPORT_PAD - width)

  const spaceBelow = vh - anchor.bottom - gap - PORTAL_LISTBOX_VIEWPORT_PAD - bottomBar
  const spaceAbove = anchor.top - gap - PORTAL_LISTBOX_VIEWPORT_PAD

  const maxBelow = Math.min(preferredMaxHeight, Math.max(PORTAL_LISTBOX_MIN_MENU_HEIGHT, spaceBelow))
  const maxAbove = Math.min(preferredMaxHeight, Math.max(PORTAL_LISTBOX_MIN_MENU_HEIGHT, spaceAbove))

  const preferBelow = spaceBelow >= PORTAL_LISTBOX_MIN_MENU_HEIGHT || spaceBelow >= spaceAbove

  if (preferBelow) {
    return {
      position: 'fixed',
      top: anchor.bottom + gap,
      left,
      width,
      maxHeight: maxBelow,
      bottom: 'auto',
    }
  }

  return {
    position: 'fixed',
    top: 'auto',
    left,
    width,
    maxHeight: maxAbove,
    bottom: vh - anchor.top + gap,
  }
}
