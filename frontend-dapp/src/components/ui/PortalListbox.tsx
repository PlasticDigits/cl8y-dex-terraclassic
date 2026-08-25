import { useCallback, useEffect, useLayoutEffect, useMemo, useReducer, type CSSProperties, type RefObject } from 'react'
import { computePortalListboxStyle } from './portalListboxPosition'
import { getPortalListboxViewport } from '@/lib/portalListboxViewport'

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
 * Shared fixed + portal listbox positioning and outside-click handling
 * for MenuSelect and TokenSelect. Uses `visualViewport` when present plus
 * DEX tab bar / in-app chrome insets ({@link getPortalListboxViewport}) so the
 * menu does not sit on IME or Keplr URL chrome (GitLab #632). Flips above the
 * anchor when space below is tight; clamps horizontally.
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
      viewport: getPortalListboxViewport(),
      preferredMaxHeight,
      gap,
    })
  }, [open, canShow, anchorRef, preferredMaxHeight, gap])

  const dropdownStyle = useMemo(() => readStyle(), [readStyle, positionEpoch])

  useLayoutEffect(() => {
    if (!open || !canShow) return
    bumpPosition()
    const w = window
    const vv = w.visualViewport
    const onMove = () => bumpPosition()
    w.addEventListener('scroll', onMove, true)
    w.addEventListener('resize', onMove)
    vv?.addEventListener('resize', onMove)
    vv?.addEventListener('scroll', onMove)
    return () => {
      w.removeEventListener('scroll', onMove, true)
      w.removeEventListener('resize', onMove)
      vv?.removeEventListener('resize', onMove)
      vv?.removeEventListener('scroll', onMove)
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
    document.addEventListener('mousedown', onDocMouseDown)
    return () => {
      document.removeEventListener('mousedown', onDocMouseDown)
    }
  }, [open, canShow, onClose, anchorRef, dropdownRef])

  return dropdownStyle
}
