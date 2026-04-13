import type { Page } from '@playwright/test'

/**
 * Fee Tiers / Create Pair / … live under the desktop header "More" menu (see Layout + navItems).
 */
export async function clickDesktopMoreNavItem(page: Page, itemLabel: string) {
  await page
    .locator('header.app-header-shell nav.app-desktop-nav')
    .getByRole('button', { name: /^More$/i })
    .click()
  await page.getByRole('menuitem', { name: itemLabel }).click()
}
