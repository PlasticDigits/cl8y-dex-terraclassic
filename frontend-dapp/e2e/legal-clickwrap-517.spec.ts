import { expect, test } from '@playwright/test'

/**
 * GitLab #517 — with Playwright webServer `VITE_PLAYWRIGHT_E2E=true`, the Legal
 * clickwrap gate is skipped (same escape hatch as the first-visit risk modal).
 * Production builds must leave that env unset.
 */
test.describe('Legal clickwrap automation escape hatch (#517)', () => {
  test('disconnected browse is not redirected to Legal portal', async ({ page }) => {
    await page.goto('/')
    await expect(page).not.toHaveURL(/terms\.cl8y\.com/)
    await expect(page.getByRole('button', { name: /connect/i }).first()).toBeVisible()
  })

  test('smoke routes render without Legal gate overlay under VITE_PLAYWRIGHT_E2E', async ({ page }) => {
    await page.goto('/pool')
    await expect(page).not.toHaveURL(/terms\.cl8y\.com/)
    // Gate container may mount only when connected; Accept Terms must not block smoke.
    await expect(page.getByRole('button', { name: /accept terms/i })).toHaveCount(0)
  })
})
