import { test, expect } from '@playwright/test'

/**
 * GitLab #181 — opening `#trade-pair-select` must not shift the trigger or desktop workspace.
 * Needs LocalTerra + LCD + indexer (see `trade-page-responsive.spec.ts`).
 */
test.describe('Trade pair select layout stability (GitLab #181)', () => {
  test('opening pair menu does not move trigger or desktop workspace', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')

    const trigger = page.locator('#trade-pair-select')
    await expect(trigger).toBeVisible({ timeout: 90_000 })

    const box = async (selector: string) => {
      const el = page.locator(selector)
      const b = await el.boundingBox()
      expect(b, `missing box for ${selector}`).toBeTruthy()
      return b!
    }

    const beforeTrigger = await box('#trade-pair-select')
    const beforeWorkspace = await box('[data-testid="trade-desktop-workspace"]')

    await trigger.click()
    await page.locator('.token-select-dropdown').waitFor({ state: 'visible', timeout: 10_000 })

    const openTrigger = await box('#trade-pair-select')
    const openWorkspace = await box('[data-testid="trade-desktop-workspace"]')

    expect(Math.abs(openTrigger.x - beforeTrigger.x)).toBeLessThan(0.5)
    expect(Math.abs(openTrigger.y - beforeTrigger.y)).toBeLessThan(0.5)
    expect(Math.abs(openWorkspace.y - beforeWorkspace.y)).toBeLessThan(0.5)
  })

  test('phone-width pair menu stays above the tab bar (GitLab #632)', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')

    const trigger = page.locator('#trade-pair-select')
    await expect(trigger).toBeVisible({ timeout: 90_000 })
    await trigger.click()
    await page.locator('.token-select-dropdown').waitFor({ state: 'visible', timeout: 10_000 })

    const clearance = await page.evaluate(() => {
      const menu = document.querySelector('.token-select-dropdown') as HTMLElement | null
      const nav = document.querySelector<HTMLElement>('.app-mobile-nav-shell')
      const tab = nav && getComputedStyle(nav).display !== 'none' ? Math.ceil(nav.getBoundingClientRect().height) : 56
      return {
        bottom: menu?.getBoundingClientRect().bottom ?? -1,
        reserved: window.innerHeight - tab - 44,
      }
    })
    expect(clearance.bottom).toBeGreaterThan(0)
    expect(clearance.bottom).toBeLessThanOrEqual(clearance.reserved + 1)
    await page.keyboard.press('Escape')
  })
})
