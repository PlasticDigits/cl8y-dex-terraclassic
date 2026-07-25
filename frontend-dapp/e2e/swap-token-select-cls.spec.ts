import { test, expect } from '@playwright/test'
import { payTokenTrigger, waitForPayTokenTriggerEnabled } from './helpers/token-select'

/**
 * GitLab #498 — opening Swap pay TokenSearchSelect on a phone-width viewport must not
 * shift the trigger or shove the pay amount field (logo/padding stay reserved).
 * Needs LocalTerra + LCD (factory tokens) like other Swap smoke specs.
 */
test.describe('Swap token select layout stability (GitLab #498)', () => {
  test('opening pay token menu does not move trigger or amount input (mobile)', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/swap')
    await page.waitForLoadState('networkidle')
    await waitForPayTokenTriggerEnabled(page, 90_000)

    const trigger = payTokenTrigger(page)
    const amount = page.locator('input.swap-io-amount-input').first()
    await expect(amount).toBeVisible({ timeout: 30_000 })

    const box = async (locator: ReturnType<typeof page.locator> | typeof trigger) => {
      const b = await locator.boundingBox()
      expect(b, 'missing bounding box').toBeTruthy()
      return b!
    }

    const beforeTrigger = await box(trigger)
    const beforeAmount = await box(amount)
    const beforeScrollY = await page.evaluate(() => window.scrollY)

    await trigger.click()
    await page.locator('.token-select-dropdown').waitFor({ state: 'visible', timeout: 10_000 })

    const openTrigger = await box(trigger)
    const openAmount = await box(amount)

    expect(Math.abs(openTrigger.x - beforeTrigger.x)).toBeLessThan(0.5)
    expect(Math.abs(openTrigger.y - beforeTrigger.y)).toBeLessThan(0.5)
    expect(Math.abs(openTrigger.width - beforeTrigger.width)).toBeLessThan(1)
    expect(Math.abs(openAmount.y - beforeAmount.y)).toBeLessThan(0.5)

    await page.keyboard.press('Escape')
    await expect(page.locator('.token-select-dropdown')).toHaveCount(0)

    const afterTrigger = await box(trigger)
    const afterAmount = await box(amount)
    const afterScrollY = await page.evaluate(() => window.scrollY)

    expect(Math.abs(afterTrigger.x - beforeTrigger.x)).toBeLessThan(0.5)
    expect(Math.abs(afterTrigger.y - beforeTrigger.y)).toBeLessThan(0.5)
    expect(Math.abs(afterAmount.y - beforeAmount.y)).toBeLessThan(0.5)
    expect(Math.abs(afterScrollY - beforeScrollY)).toBeLessThan(2)
  })
})
