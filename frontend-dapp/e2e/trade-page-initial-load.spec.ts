import { test, expect } from '@playwright/test'

/**
 * Trade hard-reload skeleton + progressive paint (GitLab #179).
 * Requires LocalTerra stack like other trade E2E suites.
 */
test.describe('Trade page initial load (GitLab #179)', () => {
  test('shows workspace skeleton immediately after hard reload', async ({ page }) => {
    await page.goto('/trade')
    await page.waitForLoadState('domcontentloaded')

    await page.reload({ waitUntil: 'commit' })

    const skeleton = page.getByTestId('trade-workspace-skeleton')
    const bootstrap = page.locator('#trade-bootstrap-shell')

    await expect(async () => {
      const skeletonVisible = await skeleton.isVisible().catch(() => false)
      const bootstrapVisible = await bootstrap.isVisible().catch(() => false)
      expect(skeletonVisible || bootstrapVisible).toBe(true)
    }).toPass({ timeout: 5_000 })
  })

  test('legal footer notice is deferred until trade workspace is ready', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    await page.goto('/trade', { waitUntil: 'commit' })

    const legalNotice = page.locator('.app-legal-footer-notice')
    const skeleton = page.getByTestId('trade-workspace-skeleton')

    await expect(async () => {
      const skeletonCount = await skeleton.count()
      const noticeVisible = await legalNotice
        .first()
        .isVisible()
        .catch(() => false)
      if (skeletonCount > 0 && (await skeleton.first().isVisible())) {
        expect(noticeVisible).toBe(false)
      }
    }).toPass({ timeout: 8_000 })

    await expect(async () => {
      await expect(page.getByRole('heading', { name: 'Trade' })).toBeVisible()
      await expect(legalNotice.first()).toBeVisible()
    }).toPass({ timeout: 90_000 })
  })
})
