import { test, expect } from './fixtures/dev-wallet'

test.describe('Portfolio page (GitLab #212, #217, #674)', () => {
  test('disconnected visit shows connect prompt', async ({ page }) => {
    await page.goto('/portfolio')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByTestId('portfolio-connect-prompt')).toBeVisible()
    await expect(page.getByRole('heading', { name: /my portfolio/i })).toBeVisible()
  })

  test('connected wallet loads portfolio shell', async ({ page, connectWallet }) => {
    await connectWallet
    await page.goto('/portfolio')
    // Portfolio polls indexer + LP LCD fan-out; networkidle never settles (GitLab #212).
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByTestId('portfolio-positions-section')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('portfolio-open-limits-section')).toBeVisible()
    await expect(page.getByTestId('portfolio-lp-overview-section')).toBeVisible()
    await expect(page.getByTestId('portfolio-recent-activity')).toBeVisible()
  })

  test('nav Portfolio link reaches route', async ({ page, connectWallet }) => {
    await connectWallet
    await page.setViewportSize({ width: 1280, height: 800 })
    await page.goto('/')
    await page.waitForLoadState('domcontentloaded')
    await page.locator('header.app-header-shell nav.app-desktop-nav').getByRole('link', { name: 'Portfolio' }).click()
    await expect(page).toHaveURL(/\/portfolio$/)
    await expect(page.getByRole('heading', { name: /my portfolio/i })).toBeVisible()
  })

  test('disconnected visit does not offer Show test pairs (GitLab #674)', async ({ page }) => {
    await page.goto('/portfolio')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByTestId('portfolio-connect-prompt')).toBeVisible()
    await expect(page.getByTestId('portfolio-show-test-pairs')).toHaveCount(0)
  })

  test('/my-portfolio redirects to /portfolio', async ({ page }) => {
    await page.goto('/my-portfolio')
    await expect(page).toHaveURL(/\/portfolio$/)
  })
})
