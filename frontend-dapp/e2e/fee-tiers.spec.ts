import { test, expect } from './fixtures/dev-wallet'

test.describe('Fee Tiers Page', () => {
  test('shows Fee Discount Tiers heading', async ({ page }) => {
    await page.goto('/tiers')
    await expect(page.getByRole('heading', { name: /Fee Discount Tiers/i })).toBeVisible()
  })

  test('shows description about CL8Y tokens', async ({ page }) => {
    await page.goto('/tiers')
    await expect(page.getByText(/Hold CL8Y tokens/i)).toBeVisible()
  })

  test('displays all 5 public fee tiers', async ({ page }) => {
    await page.goto('/tiers')
    await page.waitForLoadState('networkidle')

    await expect(page.getByText('Tier 1')).toBeVisible()
    await expect(page.getByText('Tier 2')).toBeVisible()
    await expect(page.getByText('Tier 3')).toBeVisible()
    await expect(page.getByText('Tier 4')).toBeVisible()
    await expect(page.getByText('Tier 5')).toBeVisible()
  })

  test('shows discount percentages for each tier', async ({ page }) => {
    await page.goto('/tiers')
    await page.waitForLoadState('networkidle')

    await expect(page.getByText('10%')).toBeVisible()
    await expect(page.getByText('25%')).toBeVisible()
    await expect(page.getByText('35%')).toBeVisible()
    await expect(page.getByText('50%')).toBeVisible()
    await expect(page.getByText('80%')).toBeVisible()
  })

  test('shows CL8Y holding requirements', async ({ page }) => {
    await page.goto('/tiers')
    await page.waitForLoadState('networkidle')

    await expect(page.getByText(/Hold.*CL8Y/i).first()).toBeVisible()
  })

  test('shows How It Works section', async ({ page }) => {
    await page.goto('/tiers')
    await expect(page.getByRole('heading', { name: /How it works/i })).toBeVisible()
  })

  test('shows CL8Y hold requirements in How It Works table', async ({ page }) => {
    await page.goto('/tiers')
    await page.waitForLoadState('networkidle')

    await expect(page.getByText(/CL8Y Hold/i)).toBeVisible()
    await expect(page.getByText(/drop below.*lose your tier/i)).toBeVisible()
  })

  test('shows CL8Y hold amounts per tier', async ({ page }) => {
    await page.goto('/tiers')
    await page.waitForLoadState('networkidle')

    await expect(page.getByText('1,000')).toBeVisible()
    await expect(page.getByText('15,000')).toBeVisible()
    await expect(page.getByText(/Hold \d+ CL8Y/)).toBeVisible()
  })

  test('prompts to connect wallet for registration', async ({ page }) => {
    await page.goto('/tiers')
    await page.waitForLoadState('networkidle')

    await expect(page.getByText(/Connect your wallet/i)).toBeVisible()
  })

  test.describe('With wallet connected', () => {
    test('shows register buttons for tiers', async ({ page, connectWallet }) => {
      await connectWallet
      await page.goto('/tiers')
      await page.waitForLoadState('networkidle')

      await expect(async () => {
        const registerBtns = page.getByRole('button', { name: /Register/i })
        const count = await registerBtns.count()
        expect(count).toBeGreaterThanOrEqual(1)
      }).toPass({ timeout: 10000 })
    })
  })
})
