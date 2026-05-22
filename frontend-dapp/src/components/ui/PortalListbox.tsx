import { useCallback, useEffect, useLayoutEffect, useMemo, useReducer, type CSSProperties, type RefObject } from 'react'
import { getMobileBottomNavInsetPx } from '@/lib/mobileBottomNav'
import { computePortalListboxStyle } from './portalListboxPosition'

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
 * so the menu does not collide with fixed footers or the mobile tab bar
 * ({@link getMobileBottomNavInsetPx}); clamps horizontally.
 *
 * Position is computed synchronously during render when open so the first painted
 * frame already uses `position: fixed` coords (avoids CLS from a late setState pass).
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
  const [positionEpoch, bumpPosition] = useReducer((n: number) => n + 1, 0)

  const readStyle = useCallback((): CSSProperties | null => {
    if (!open || !canShow) return null
    const el = anchorRef.current
    if (!el) return null
    return computePortalListboxStyle({
      anchor: el.getBoundingClientRect(),
      viewport: {
        width: window.innerWidth,
        height: window.innerHeight,
        bottomInset: getMobileBottomNavInsetPx(),
      },
      preferredMaxHeight,
      gap,
    })
  }, [open, canShow, anchorRef, preferredMaxHeight, gap])

  const dropdownStyle = useMemo(() => readStyle(), [readStyle, positionEpoch])

  useLayoutEffect(() => {
    if (!open || !canShow) return
    bumpPosition()
    const w = window
    const onMove = () => bumpPosition()
    w.addEventListener('scroll', onMove, true)
    w.addEventListener('resize', onMove)
    return () => {
      w.removeEventListener('scroll', onMove, true)
      w.removeEventListener('resize', onMove)
    }
  }, [open, canShow])

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

  return dropdownStyle
}
