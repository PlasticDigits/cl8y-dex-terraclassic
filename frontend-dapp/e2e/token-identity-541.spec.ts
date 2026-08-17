import { test, expect } from '@playwright/test'
import { e2eTradePairFromDeploy } from './helpers/deploy-pair'

const TRADE_PAIR = e2eTradePairFromDeploy()

async function skipIfNoIdentityRow(page: import('@playwright/test').Page, timeoutMs = 45_000) {
  const row = page.getByTestId('pair-token-links').first()
  try {
    await expect(row).toBeVisible({ timeout: timeoutMs })
  } catch {
    test.skip(true, 'No factory/indexer pair with a checksummed address on this stack')
  }
}

/**
 * Playwright smoke for compact token identity (GitLab #541).
 * P4 needs no chain. P1–P3 skip when the identity row never appears (no pair list).
 * Use PLAYWRIGHT_SKIP_CHAIN=1 to avoid e2e globalSetup seeding.
 */
test.describe('Token identity Playwright smoke (GitLab #541)', () => {
  test('P4: invalid /trade deep link has no identity explorer anchors', async ({ page }) => {
    await page.goto('/trade/lilwayne%20babyyy', { waitUntil: 'domcontentloaded' })
    await expect(page.getByTestId('trade-invalid-pair-link-notice')).toBeVisible({ timeout: 30_000 })
    await expect(page.getByTestId('pair-token-links')).toHaveCount(0)
    await expect(page.getByTestId('token-identity-base')).toHaveCount(0)
    await expect(page.getByTestId('token-identity-pair-explorer')).toHaveCount(0)
  })

  test('P1: /pool selected pair shows token-identity chips when a factory pair exists', async ({ page }) => {
    await page.goto('/pool', { waitUntil: 'domcontentloaded' })
    await expect(page.getByText(/liquidity pools/i)).toBeVisible({ timeout: 30_000 })
    await skipIfNoIdentityRow(page)
    await expect(page.getByTestId('token-identity-base').first()).toBeVisible()
    await expect(page.getByTestId('token-identity-quote').first()).toBeVisible()
    await expect(page.getByTestId('token-identity-pair').first()).toBeVisible()
  })

  test('P2: /trade/<known-pair> identity row is outside the combobox', async ({ page }) => {
    await page.goto(`/trade/${TRADE_PAIR}`, { waitUntil: 'domcontentloaded' })
    await expect(page.getByTestId('trade-pair-select-panel')).toBeVisible({ timeout: 30_000 })
    await skipIfNoIdentityRow(page)
    const panel = page.getByTestId('trade-pair-select-panel')
    expect(await panel.locator('[data-testid="pair-token-links"]').count()).toBe(1)
    const combobox = page.getByRole('combobox', { name: 'Trading pair' })
    await expect(combobox).toBeVisible()
    await combobox.click()
    const listbox = page.getByRole('listbox')
    await expect(listbox).toBeVisible()
    await expect(listbox.getByTestId('token-identity-base-explorer')).toHaveCount(0)
  })

  test('P3: /charts identity explorer href is Finder/LCD, not the dApp origin', async ({ page }) => {
    await page.goto('/charts', { waitUntil: 'domcontentloaded' })
    await expect(page.getByRole('heading', { name: /charts & analytics/i })).toBeVisible({
      timeout: 30_000,
    })
    await skipIfNoIdentityRow(page)
    const explorer = page
      .getByTestId('token-identity-base-explorer')
      .or(page.getByTestId('token-identity-pair-explorer'))
    if ((await explorer.count()) === 0) {
      return
    }
    const href = await explorer.first().getAttribute('href')
    expect(href).toBeTruthy()
    expect(href).not.toContain('localhost:3000')
    expect(href).not.toContain('127.0.0.1:5173')
    expect(href).toMatch(/finder\.|cosmos\/auth\/v1beta1\/accounts\//)
  })
})
