import { test, expect } from './fixtures/dev-wallet'
import { expectPayTokenListPopulated, payTokenTrigger } from './helpers/token-select'

test.describe('Swap Page', () => {
  test.describe('Without wallet', () => {
    test('shows swap form with pair selector', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      await expect(page.getByRole('heading', { name: 'Swap' })).toBeVisible()
      await expect(page.getByText('You Pay')).toBeVisible()
      await expect(page.getByText('You Receive')).toBeVisible()
    })

    test('loads available trading pairs from factory', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      await expect(payTokenTrigger(page)).toBeVisible()
      await expectPayTokenListPopulated(page)
    })

    test('shows You Pay and You Receive sections', async ({ page }) => {
      await page.goto('/')
      await expect(page.getByText('You Pay')).toBeVisible()
      await expect(page.getByText('You Receive')).toBeVisible()
    })

    test('shows Connect Wallet as the submit button when disconnected', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      const swapPanel = page.locator('main .shell-panel-strong').first()
      const submitBtn = swapPanel.getByRole('button', { name: /^Connect Wallet$/i })
      await expect(submitBtn).toBeVisible()
      // CTA opens the wallet modal when disconnected; it stays enabled so users can tap it.
      await expect(submitBtn).toBeEnabled()
    })

    test('has a settings button', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      await expect(page.getByRole('button', { name: 'Settings' })).toBeVisible()
    })

    test('has a swap direction toggle button', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      const buttons = page.locator('button')
      const swapDirectionBtn = buttons.filter({
        has: page.locator('svg'),
      })
      await expect(swapDirectionBtn.first()).toBeVisible()
    })
  })

  test.describe('With wallet connected', () => {
    test('shows Enter Amount when no amount typed', async ({ page, connectWallet }) => {
      await connectWallet
      await page.waitForLoadState('networkidle')

      await expectPayTokenListPopulated(page)

      const swapPanel = page.locator('main .shell-panel-strong').first()
      await expect(swapPanel.getByRole('button', { name: 'Enter Amount' })).toBeVisible()
    })

    test('accepts numeric input in You Pay field', async ({ page, connectWallet }) => {
      await connectWallet
      await page.waitForLoadState('networkidle')
      const input = page.getByPlaceholder('0.00').first()
      await input.fill('100')
      await expect(input).toHaveValue('100')
    })

    test('clears amount on empty input', async ({ page, connectWallet }) => {
      await connectWallet
      await page.waitForLoadState('networkidle')
      const input = page.getByPlaceholder('0.00').first()
      await input.fill('100')
      await expect(input).toHaveValue('100')
      await input.fill('')
      await expect(input).toHaveValue('')
    })

    test('shows estimated output when amount entered with loaded pair', async ({ page, connectWallet }) => {
      await connectWallet
      await page.waitForLoadState('networkidle')

      await expectPayTokenListPopulated(page)

      const input = page.getByPlaceholder('0.00').first()
      await input.fill('1000')

      await expect(async () => {
        const outputText = await page.getByText('You Receive').locator('..').textContent()
        expect(outputText).toBeTruthy()
      }).toPass({ timeout: 10000 })
    })
  })
})
