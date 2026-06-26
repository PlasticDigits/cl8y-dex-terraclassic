import { test, expect } from './fixtures/dev-wallet'
import { E2E_DEV_WALLET } from './helpers/fee-discount-quote-e2e'

const CONSOLE_ERROR_ALLOWLIST = [
  /favicon/i,
  /Failed to load resource.*favicon/i,
  /404.*favicon/i,
  /Content Security Policy directive 'connect-src' contains an invalid source/i,
]

test.describe('Trader page smoke (GitLab #422)', () => {
  test('connected wallet loads positions without console errors', async ({ page, connectWallet }) => {
    const consoleErrors: string[] = []
    page.on('console', (msg) => {
      if (msg.type() === 'error') consoleErrors.push(msg.text())
    })
    page.on('pageerror', (err) => {
      consoleErrors.push(err.message)
    })

    await connectWallet
    await page.goto(`/trader/${E2E_DEV_WALLET}`)
    await page.waitForLoadState('domcontentloaded')

    await expect(page.getByRole('heading', { name: /trader profile/i })).toBeVisible({ timeout: 15_000 })

    const positionsSection = page.getByTestId('trader-positions-section')
    await expect(positionsSection).toBeVisible({ timeout: 30_000 })
    await expect(positionsSection.getByRole('heading', { name: /open positions/i })).toBeVisible()

    await expect(async () => {
      const loadingSkeleton = positionsSection.locator('[class*="skeleton"], [data-testid*="skeleton"]')
      expect(await loadingSkeleton.count()).toBe(0)
    }).toPass({ timeout: 30_000 })

    const hasRows = (await positionsSection.locator('table tbody tr').count()) > 0
    const hasEmpty = await page
      .getByTestId('trader-positions-empty')
      .isVisible()
      .catch(() => false)
    expect(hasRows || hasEmpty, 'positions section should show table rows or empty state').toBe(true)

    const unexpected = consoleErrors.filter((line) => !CONSOLE_ERROR_ALLOWLIST.some((re) => re.test(line)))
    expect(unexpected, `unexpected console errors: ${unexpected.join('; ')}`).toEqual([])
  })
})
