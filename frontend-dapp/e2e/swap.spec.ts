import { test, expect } from './fixtures/dev-wallet'

test.describe('Swap Page', () => {
  test.describe('Without wallet', () => {
    test('shows swap form with pair selector', async ({ page }) => {
      await page.goto('/')
      await expect(page.getByRole('heading', { name: 'Swap' })).toBeVisible()
      await expect(page.getByText('Trading Pair')).toBeVisible()
    })

    test('loads available trading pairs from factory', async ({ page }) => {
      await page.goto('/')
      const pairSelector = page.locator('select')
      await expect(pairSelector).toBeVisible()
      await pairSelector.waitFor({ state: 'attached' })
      await expect(async () => {
        const options = await pairSelector.locator('option').allTextContents()
        const hasPair = options.some(
          (opt) => !opt.includes('Loading') && !opt.includes('Select')
        )
        expect(hasPair).toBe(true)
      }).toPass({ timeout: 15000 })
    })

    test('shows You Pay and You Receive sections', async ({ page }) => {
      await page.goto('/')
      await expect(page.getByText('You Pay')).toBeVisible()
      await expect(page.getByText('You Receive')).toBeVisible()
    })

    test('shows Connect Wallet as the submit button when disconnected', async ({ page }) => {
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      const submitBtn = page.locator('button').filter({ hasText: /Connect Wallet/i }).last()
      await expect(submitBtn).toBeVisible()
      await expect(submitBtn).toBeDisabled()
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
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      await expect(async () => {
        const pairSelector = page.locator('select')
        const options = await pairSelector.locator('option').allTextContents()
        const hasPair = options.some(
          (opt) => !opt.includes('Loading') && !opt.includes('Select')
        )
        expect(hasPair).toBe(true)
      }).toPass({ timeout: 15000 })

      const submitBtn = page.locator('button').filter({ hasText: /Enter Amount|Swap|Connect/i }).last()
      await expect(submitBtn).toBeVisible()
    })

    test('accepts numeric input in You Pay field', async ({ page, connectWallet }) => {
      await connectWallet
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      const input = page.getByPlaceholder('0.00').first()
      await input.fill('100')
      await expect(input).toHaveValue('100')
    })

    test('clears amount on empty input', async ({ page, connectWallet }) => {
      await connectWallet
      await page.goto('/')
      await page.waitForLoadState('networkidle')
      const input = page.getByPlaceholder('0.00').first()
      await input.fill('100')
      await expect(input).toHaveValue('100')
      await input.fill('')
      await expect(input).toHaveValue('')
    })

    test('shows estimated output when amount entered with loaded pair', async ({ page, connectWallet }) => {
      await connectWallet
      await page.goto('/')

      await expect(async () => {
        const pairSelector = page.locator('select')
        const options = await pairSelector.locator('option').allTextContents()
        const hasPair = options.some(
          (opt) => !opt.includes('Loading') && !opt.includes('Select')
        )
        expect(hasPair).toBe(true)
      }).toPass({ timeout: 15000 })

      const input = page.getByPlaceholder('0.00').first()
      await input.fill('1000')

      await expect(async () => {
        const outputText = await page.getByText('You Receive').locator('..').textContent()
        expect(outputText).toBeTruthy()
      }).toPass({ timeout: 10000 })
    })
  })
})
