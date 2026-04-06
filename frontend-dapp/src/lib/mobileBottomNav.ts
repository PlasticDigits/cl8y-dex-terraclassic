/**
 * Live height of the fixed mobile bottom tab bar, or 0 when it is not shown.
 * Used by portaled/fixed UI (listboxes, modals) to avoid overlapping the bar.
 */
export function getMobileBottomNavInsetPx(): number {
  if (typeof document === 'undefined') return 0
  const nav = document.querySelector<HTMLElement>('.app-mobile-nav-shell')
  if (!nav) return 0
  if (getComputedStyle(nav).display === 'none') return 0
  return Math.ceil(nav.getBoundingClientRect().height)
}
