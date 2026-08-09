export type NavItem = {
  path: string
  label: string
  end?: boolean
}

export const PRIMARY_NAV_ITEMS: NavItem[] = [
  { path: '/', label: 'Swap', end: true },
  { path: '/pool', label: 'Pool' },
  { path: '/limits', label: 'Limits' },
  { path: '/trade', label: 'Trade', end: false },
  { path: '/portfolio', label: 'Portfolio' },
  { path: '/charts', label: 'Charts' },
]

export const MORE_NAV_ITEMS: NavItem[] = [
  { path: '/trader', label: 'Trader', end: false },
  { path: '/protocol', label: 'Protocol' },
  { path: '/tiers', label: 'Fee Tiers' },
  { path: '/create', label: 'Create Pair' },
]

/** Soft-launch faucet route (GitLab #473) — appended to More menus only when enabled. */
export const MINT_NAV_ITEM: NavItem = { path: '/mint', label: 'Mint' }

/**
 * ust1-window oracle mint/redeem (GitLab #506) — label must never be "Mint" (faucet).
 * Appended to More menus only when window env is configured.
 */
export const UST1_NAV_ITEM: NavItem = { path: '/ust1', label: 'UST1' }

/**
 * Native LUNC/USTC ↔ cLUNC/cUSTC wrap (GitLab #507 / #502) — More menu when wrap env is set.
 * Distinct from Swap AMM paths and from UST1 oracle mint.
 */
export const WRAP_NAV_ITEM: NavItem = { path: '/wrap', label: 'Wrap' }

export type NavMenuOptions = {
  includeMint?: boolean
  includeUst1?: boolean
  includeWrap?: boolean
}

function appendConditionalNavItems(items: NavItem[], options?: NavMenuOptions): NavItem[] {
  let next = items
  if (options?.includeUst1) {
    next = [...next, UST1_NAV_ITEM]
  }
  if (options?.includeWrap) {
    next = [...next, WRAP_NAV_ITEM]
  }
  if (options?.includeMint) {
    next = [...next, MINT_NAV_ITEM]
  }
  return next
}

/**
 * Minimum viewport width (px) for the sticky header to show every primary link inline.
 * Between **768px** and **this − 1**, Pool/Limits/Trade/Charts fold into the header **More** menu
 * so mid-range tablets avoid a cramped single row ([GitLab #136](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)).
 *
 * Raised from 1024 → 1120 after follow-up (full primary row still overlapped wallet/controls at ~1024–1098px),
 * then 1120 → 1200 so More vs theme/wallet keeps ≥ ~8px after theme moved into the header ([GitLab #483](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/483)).
 */
export const HEADER_FULL_NAV_MIN_WIDTH_PX = 1200

/** Last viewport width (px) that uses tablet-compact header nav (Swap + More inline). */
export const TABLET_COMPACT_HEADER_MAX_WIDTH_PX = HEADER_FULL_NAV_MIN_WIDTH_PX - 1

/** Visible left-to-right order in `header nav.app-desktop-nav` when viewport ≥ `HEADER_FULL_NAV_MIN_WIDTH_PX`. */
export const DESKTOP_HEADER_NAV_ROW_LABELS = [...PRIMARY_NAV_ITEMS.map((item) => item.label), 'More'] as const

/** Inline header labels when viewport is 768px–`TABLET_COMPACT_HEADER_MAX_WIDTH_PX` (desktop nav visible, compact row). */
export const TABLET_COMPACT_HEADER_NAV_ROW_LABELS = ['Swap', 'More'] as const

export function getHeaderMoreMenuItems(fullDesktopHeader: boolean, options?: NavMenuOptions): NavItem[] {
  const base = fullDesktopHeader ? MORE_NAV_ITEMS : [...PRIMARY_NAV_ITEMS.slice(1), ...MORE_NAV_ITEMS]
  return appendConditionalNavItems(base, options)
}

/** Bottom tab bar on viewports ≤767px — core trade routes only (GitLab #347). */
export const MOBILE_BOTTOM_NAV_ITEMS: NavItem[] = [
  { path: '/', label: 'Swap', end: true },
  { path: '/trade', label: 'Trade', end: false },
  { path: '/pool', label: 'Pool' },
  { path: '/limits', label: 'Limits' },
]

/** Overflow primaries + secondary routes in the mobile More sheet. */
export function getMobileMoreMenuItems(options?: NavMenuOptions): NavItem[] {
  const bottomPaths = new Set(MOBILE_BOTTOM_NAV_ITEMS.map((item) => item.path))
  const overflowPrimaries = PRIMARY_NAV_ITEMS.filter((item) => !bottomPaths.has(item.path))
  return appendConditionalNavItems([...overflowPrimaries, ...MORE_NAV_ITEMS], options)
}
