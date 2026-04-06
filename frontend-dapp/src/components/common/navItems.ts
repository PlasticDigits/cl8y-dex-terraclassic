export type NavItem = {
  path: string
  label: string
  end?: boolean
}

export const PRIMARY_NAV_ITEMS: NavItem[] = [
  { path: '/', label: 'Swap', end: true },
  { path: '/pool', label: 'Pool' },
  { path: '/limits', label: 'Limits' },
  { path: '/charts', label: 'Charts' },
]

export const MORE_NAV_ITEMS: NavItem[] = [
  { path: '/trader', label: 'Trader', end: false },
  { path: '/protocol', label: 'Protocol' },
  { path: '/tiers', label: 'Fee Tiers' },
  { path: '/create', label: 'Create Pair' },
]

/** Visible left-to-right order in `header nav.app-desktop-nav` (labels + More trigger). */
export const DESKTOP_HEADER_NAV_ROW_LABELS = [...PRIMARY_NAV_ITEMS.map((item) => item.label), 'More'] as const
