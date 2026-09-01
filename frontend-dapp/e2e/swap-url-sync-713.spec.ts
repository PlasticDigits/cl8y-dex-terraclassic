import { test, expect } from '@playwright/test'
import { payTokenTrigger, receiveTokenTrigger, waitForPayTokenTriggerEnabled } from './helpers/token-select'

/**
 * GitLab #713 — Swap URL rewrite + Share. PLAYWRIGHT_SKIP_CHAIN=1 is OK when LCD serves pairs.
 */
test.describe('Swap URL sync (GitLab #713)', () => {
  test('token change rewrites the bar to canonical from/to', async ({ page }) => {
    await page.goto('/?from=uluna&to=uusd')
    await page.waitForLoadState('networkidle')
    await waitForPayTokenTriggerEnabled(page, 90_000)
    await expect(payTokenTrigger(page)).toContainText(/LUNC/i)
    await expect(receiveTokenTrigger(page)).toContainText(/USTC/i)
    const url = new URL(page.url())
    expect(url.search).toMatch(/from=LUNC/)
    expect(url.search).toMatch(/to=USTC/)
    expect(url.search).not.toMatch(/inputCurrency/)
  })

  test('Share control is present on Swap header', async ({ page }) => {
    await page.goto('/?from=uluna&to=uusd')
    await page.waitForLoadState('networkidle')
    await waitForPayTokenTriggerEnabled(page, 90_000)
    await expect(page.getByTestId('swap-share-link')).toBeVisible()
    await expect(page.getByTestId('swap-share-link')).toHaveAttribute('aria-label', /Share LUNC to USTC swap link/i)
  })

  test('/create?a=&b= does not auto-submit Create Pair', async ({ page }) => {
    const a = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
    const b = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
    await page.goto(`/create?a=${a}&b=${b}`)
    await expect(page.getByRole('heading', { name: /create trading pair/i })).toBeVisible({
      timeout: 20_000,
    })
    await expect(
      page.getByRole('button', { name: /Connect Wallet To Create|Creating Pair|Create Pair/i })
    ).toBeVisible()
  })
})
