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
  { path: '/portfolio', label: 'Portfolio' },
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
 *
 * Raised from 1024 → 1120 after follow-up: full primary row still overlapped wallet/controls at ~1024–1098px.
 */
export const HEADER_FULL_NAV_MIN_WIDTH_PX = 1120

/** Last viewport width (px) that uses tablet-compact header nav (Swap + More inline). */
export const TABLET_COMPACT_HEADER_MAX_WIDTH_PX = HEADER_FULL_NAV_MIN_WIDTH_PX - 1

/** Visible left-to-right order in `header nav.app-desktop-nav` when viewport ≥ `HEADER_FULL_NAV_MIN_WIDTH_PX`. */
export const DESKTOP_HEADER_NAV_ROW_LABELS = [...PRIMARY_NAV_ITEMS.map((item) => item.label), 'More'] as const

/** Inline header labels when viewport is 768px–`TABLET_COMPACT_HEADER_MAX_WIDTH_PX` (desktop nav visible, compact row). */
export const TABLET_COMPACT_HEADER_NAV_ROW_LABELS = ['Swap', 'More'] as const

export function getHeaderMoreMenuItems(fullDesktopHeader: boolean): NavItem[] {
  return fullDesktopHeader ? MORE_NAV_ITEMS : [...PRIMARY_NAV_ITEMS.slice(1), ...MORE_NAV_ITEMS]
}

/** Bottom tab bar on viewports ≤767px — core trade routes only (GitLab #347). */
export const MOBILE_BOTTOM_NAV_ITEMS: NavItem[] = [
  { path: '/', label: 'Swap', end: true },
  { path: '/trade', label: 'Trade', end: false },
  { path: '/pool', label: 'Pool' },
  { path: '/limits', label: 'Limits' },
]

/** Overflow primaries + secondary routes in the mobile More sheet. */
export function getMobileMoreMenuItems(): NavItem[] {
  const bottomPaths = new Set(MOBILE_BOTTOM_NAV_ITEMS.map((item) => item.path))
  const overflowPrimaries = PRIMARY_NAV_ITEMS.filter((item) => !bottomPaths.has(item.path))
  return [...overflowPrimaries, ...MORE_NAV_ITEMS]
}
