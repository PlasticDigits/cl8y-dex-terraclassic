import { test, expect } from '@playwright/test'
import { payTokenTrigger, receiveTokenTrigger, waitForPayTokenTriggerEnabled } from './helpers/token-select'

/**
 * GitLab #715 — tokenlist symbols in Swap from=/to= + Share logos.
 * PLAYWRIGHT_SKIP_CHAIN=1 is OK when LCD serves pairs. 5 workers (e2e-smoke).
 */
test.describe('Swap tokenlist symbols (GitLab #715)', () => {
  test.use({ permissions: ['clipboard-read', 'clipboard-write'] })

  test('/?from=LUNC&to=USTC and /swap?from=uluna rewrite to symbols', async ({ page }) => {
    await page.goto('/?from=LUNC&to=USTC')
    await page.waitForLoadState('networkidle')
    await waitForPayTokenTriggerEnabled(page, 90_000)
    await expect(payTokenTrigger(page)).toContainText(/LUNC/i)
    await expect(receiveTokenTrigger(page)).toContainText(/USTC/i)
    expect(new URL(page.url()).search).toMatch(/from=LUNC/)
    expect(new URL(page.url()).search).toMatch(/to=USTC/)

    await page.goto('/swap?from=uluna&to=uusd')
    await page.waitForLoadState('networkidle')
    await waitForPayTokenTriggerEnabled(page, 90_000)
    await expect(payTokenTrigger(page)).toContainText(/LUNC/i)
    expect(new URL(page.url()).pathname).toBe('/')
    expect(new URL(page.url()).search).toMatch(/from=LUNC/)
    expect(new URL(page.url()).search).toMatch(/to=USTC/)
  })

  test('hostile from= is ignored and not echoed', async ({ page }) => {
    await page.goto('/?from=javascript:alert(1)&to=USTC')
    await page.waitForLoadState('networkidle')
    await waitForPayTokenTriggerEnabled(page, 90_000)
    await expect(payTokenTrigger(page)).not.toContainText(/javascript/i)
    expect(page.url()).not.toMatch(/javascript/i)
  })

  test('Share control shows pair logos and copies a symbol URL', async ({ page }) => {
    await page.goto('/?from=LUNC&to=USTC')
    await page.waitForLoadState('networkidle')
    await waitForPayTokenTriggerEnabled(page, 90_000)
    const share = page.getByTestId('swap-share-link')
    await expect(share).toBeVisible()
    await expect(share).toHaveAttribute('aria-label', /Share LUNC to USTC swap link/i)
    await expect(share.locator('img')).toHaveCount(2)
    await share.click()
    const copied = await page.evaluate(async () => navigator.clipboard.readText())
    expect(copied).toMatch(/from=LUNC/)
    expect(copied).toMatch(/to=USTC/)
    expect(copied).not.toMatch(/uluna|uusd/)

    await page.getByRole('button', { name: 'Swap pay and receive tokens' }).click()
    await expect(share).toHaveAttribute('aria-label', /Share USTC to LUNC swap link/i)
  })
})
