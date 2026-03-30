import { useCallback, useEffect, useLayoutEffect, useState, type CSSProperties, type RefObject } from 'react'

const VIEWPORT_PAD = 8
const MIN_MENU_HEIGHT = 120

export type UsePortalListboxArgs = {
  open: boolean
  /** When false, positioning is cleared (e.g. no options). */
  canShow: boolean
  anchorRef: RefObject<HTMLElement | null>
  dropdownRef: RefObject<HTMLElement | null>
  onClose: () => void
  /** Cap for list max-height (TokenSelect vs MenuSelect). */
  preferredMaxHeight?: number
  gap?: number
}

/**
 * Shared fixed + portal listbox positioning and outside-click / Escape handling
 * for MenuSelect and TokenSelect. Flips above the anchor when space below is tight
 * so the menu does not collide with fixed footers or bottom nav; clamps horizontally.
 */
export function usePortalListbox({
  open,
  canShow,
  anchorRef,
  dropdownRef,
  onClose,
  preferredMaxHeight = 280,
  gap = 8,
}: UsePortalListboxArgs): CSSProperties | null {
  const [dropdownStyle, setDropdownStyle] = useState<CSSProperties | null>(null)

  const updatePosition = useCallback(() => {
    const el = anchorRef.current
    if (!el) return

    const r = el.getBoundingClientRect()
    const vw = window.innerWidth
    const vh = window.innerHeight

    const width = Math.min(r.width, vw - 2 * VIEWPORT_PAD)
    let left = r.left + (r.width - width) / 2
    left = Math.min(Math.max(VIEWPORT_PAD, left), vw - VIEWPORT_PAD - width)

    const spaceBelow = vh - r.bottom - gap - VIEWPORT_PAD
    const spaceAbove = r.top - gap - VIEWPORT_PAD

    const maxBelow = Math.min(preferredMaxHeight, Math.max(MIN_MENU_HEIGHT, spaceBelow))
    const maxAbove = Math.min(preferredMaxHeight, Math.max(MIN_MENU_HEIGHT, spaceAbove))

    const preferBelow = spaceBelow >= MIN_MENU_HEIGHT || spaceBelow >= spaceAbove

    if (preferBelow) {
      setDropdownStyle({
        position: 'fixed',
        top: r.bottom + gap,
        left,
        width,
        maxHeight: maxBelow,
        bottom: 'auto',
      })
    } else {
      setDropdownStyle({
        position: 'fixed',
        top: 'auto',
        left,
        width,
        maxHeight: maxAbove,
        bottom: vh - r.top + gap,
      })
    }
  }, [anchorRef, gap, preferredMaxHeight])

  useLayoutEffect(() => {
    if (!open || !canShow) {
      setDropdownStyle(null)
      return
    }
    updatePosition()
    const w = window
    w.addEventListener('scroll', updatePosition, true)
    w.addEventListener('resize', updatePosition)
    return () => {
      w.removeEventListener('scroll', updatePosition, true)
      w.removeEventListener('resize', updatePosition)
    }
  }, [open, canShow, updatePosition])

  useEffect(() => {
    if (!open || !canShow) return
    function onDocMouseDown(e: MouseEvent) {
      const t = e.target as Node
      if (anchorRef.current?.contains(t)) return
      if (dropdownRef.current?.contains(t)) return
      onClose()
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('mousedown', onDocMouseDown)
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('mousedown', onDocMouseDown)
      document.removeEventListener('keydown', onKey)
    }
  }, [open, canShow, onClose, anchorRef, dropdownRef])

  return open && canShow ? dropdownStyle : null
}
