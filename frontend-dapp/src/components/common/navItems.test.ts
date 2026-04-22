import { describe, expect, it } from 'vitest'
import { DESKTOP_HEADER_NAV_ROW_LABELS, MORE_NAV_ITEMS, PRIMARY_NAV_ITEMS } from '@/components/common/navItems'

describe('navItems', () => {
  it('desktop header row labels match primary nav plus More trigger', () => {
    expect(DESKTOP_HEADER_NAV_ROW_LABELS).toEqual([...PRIMARY_NAV_ITEMS.map((item) => item.label), 'More'])
  })

  it('has expected primary and more routes for regression checks', () => {
    expect(PRIMARY_NAV_ITEMS.map((i) => i.path)).toEqual(['/', '/pool', '/limits', '/trade', '/charts'])
    expect(MORE_NAV_ITEMS.length).toBeGreaterThan(0)
  })
})
