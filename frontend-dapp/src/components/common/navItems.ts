export type NavItem = {
  path: string
  label: string
  end?: boolean
}

export const PRIMARY_NAV_ITEMS: NavItem[] = [
  { path: '/', label: 'Swap', end: true },
  { path: '/pool', label: 'Pool' },
  { path: '/limits', label: 'Limits' },
  { path: '/trade', label: 'Trade' },
  { path: '/charts', label: 'Charts' },
]

export const MORE_NAV_ITEMS: NavItem[] = [
  { path: '/trader', label: 'Trader', end: false },
  { path: '/protocol', label: 'Protocol' },
  { path: '/tiers', label: 'Fee Tiers' },
  { path: '/create', label: 'Create Pair' },
]

/**
 * Minimum viewport width (px) for the sticky header to show every primary link inline.
 * Between **768px** and **this − 1**, Pool/Limits/Trade/Charts fold into the header **More** menu
 * so mid-range tablets avoid a cramped single row ([GitLab #136](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)).
 */
export const HEADER_FULL_NAV_MIN_WIDTH_PX = 1024

/** Visible left-to-right order in `header nav.app-desktop-nav` when viewport ≥ `HEADER_FULL_NAV_MIN_WIDTH_PX`. */
export const DESKTOP_HEADER_NAV_ROW_LABELS = [...PRIMARY_NAV_ITEMS.map((item) => item.label), 'More'] as const

/** Inline header labels when viewport is 768px–1023px (desktop nav visible, compact row). */
export const TABLET_COMPACT_HEADER_NAV_ROW_LABELS = ['Swap', 'More'] as const

export function getHeaderMoreMenuItems(fullDesktopHeader: boolean): NavItem[] {
  return fullDesktopHeader ? MORE_NAV_ITEMS : [...PRIMARY_NAV_ITEMS.slice(1), ...MORE_NAV_ITEMS]
}
