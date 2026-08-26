import { test, expect } from './fixtures/dev-wallet'
import { E2E_DEV_WALLET } from './helpers/fee-discount-quote-e2e'
import type { Page, Locator } from '@playwright/test'

const CONSOLE_ERROR_ALLOWLIST = [
  /favicon/i,
  /Failed to load resource.*favicon/i,
  /404.*favicon/i,
  /Content Security Policy directive 'connect-src' contains an invalid source/i,
  /ERR_CONNECTION_REFUSED/i,
]

async function waitForLeaderboardSettled(page: Page): Promise<Locator> {
  const board = page.getByTestId('trader-leaderboard')
  await expect(board).toBeVisible({ timeout: 15_000 })
  await expect(board.getByRole('heading', { name: /^leaderboard$/i })).toBeVisible()
  await expect(async () => {
    const hasTable = await board.getByRole('table', { name: /trader leaderboard/i }).isVisible()
    const hasEmpty = await board.getByText(/no traders yet/i).isVisible()
    const hasRetry = await board.getByTestId('retry-error-button').isVisible()
    expect(hasTable || hasEmpty || hasRetry).toBe(true)
  }).toPass({ timeout: 30_000 })
  return board
}

async function expectBoardAboveFooter(page: Page, board: Locator) {
  await board.scrollIntoViewIfNeeded()
  const footer = page.locator('footer.app-footer-shell')
  await expect(footer).toBeVisible()
  const boardBox = await board.boundingBox()
  const footerBox = await footer.boundingBox()
  expect(boardBox, 'leaderboard bounding box').toBeTruthy()
  expect(footerBox, 'layout footer bounding box').toBeTruthy()
  expect(boardBox!.y).toBeLessThan(footerBox!.y)
}

test.describe('Trader page smoke (GitLab #422 / #657)', () => {
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

    const board = await waitForLeaderboardSettled(page)
    await expectBoardAboveFooter(page, board)

    const outage = page.getByTestId('trader-market-data-outage-banner')
    const profileOutage = await outage.isVisible().catch(() => false)
    if (!profileOutage) {
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
    }

    const unexpected = consoleErrors.filter((line) => !CONSOLE_ERROR_ALLOWLIST.some((re) => re.test(line)))
    expect(unexpected, `unexpected console errors: ${unexpected.join('; ')}`).toEqual([])
  })

  test('empty /trader shows Leaderboard above the layout footer', async ({ page }) => {
    await page.goto('/trader')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByRole('heading', { name: /trader profile/i })).toBeVisible({ timeout: 15_000 })
    await expect(page.getByText(/search for a trader wallet/i)).toBeVisible()
    const board = await waitForLeaderboardSettled(page)
    await expectBoardAboveFooter(page, board)
  })
})

test.describe('Trader profile Share (GitLab #665)', () => {
  test('Share control is visible on a valid trader path', async ({ page, connectWallet }) => {
    await connectWallet
    await page.goto(`/trader/${E2E_DEV_WALLET}`)
    await page.waitForLoadState('domcontentloaded')

    await expect(page.getByRole('heading', { name: /trader profile/i })).toBeVisible({ timeout: 15_000 })
    const share = page.getByTestId('trader-share-link')
    await expect(share).toBeVisible({ timeout: 15_000 })
    await expect(share).toHaveAttribute('aria-label', 'Share trader profile link')
    await expect(share).toContainText('Share')
  })
})
