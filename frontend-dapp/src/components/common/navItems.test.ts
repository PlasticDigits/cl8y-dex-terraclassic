import { describe, expect, it } from 'vitest'
import {
  DESKTOP_HEADER_NAV_ROW_LABELS,
  getHeaderMoreMenuItems,
  MORE_NAV_ITEMS,
  PRIMARY_NAV_ITEMS,
  TABLET_COMPACT_HEADER_NAV_ROW_LABELS,
} from '@/components/common/navItems'

describe('navItems', () => {
  it('desktop header row labels match primary nav plus More trigger', () => {
    expect(DESKTOP_HEADER_NAV_ROW_LABELS).toEqual([...PRIMARY_NAV_ITEMS.map((item) => item.label), 'More'])
  })

  it('tablet compact header shows Swap plus More only', () => {
    expect(TABLET_COMPACT_HEADER_NAV_ROW_LABELS).toEqual(['Swap', 'More'])
  })

  it('merges overflow primaries into More below full-desktop breakpoint', () => {
    expect(getHeaderMoreMenuItems(true)).toEqual(MORE_NAV_ITEMS)
    expect(getHeaderMoreMenuItems(false)).toEqual([...PRIMARY_NAV_ITEMS.slice(1), ...MORE_NAV_ITEMS])
  })

  it('has expected primary and more routes for regression checks', () => {
    expect(PRIMARY_NAV_ITEMS.map((i) => i.path)).toEqual(['/', '/pool', '/limits', '/trade', '/charts'])
    expect(MORE_NAV_ITEMS.length).toBeGreaterThan(0)
  })
})
