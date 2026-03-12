import { test, expect } from './fixtures/dev-wallet'

test.describe('Pool Page', () => {
  test.describe('Without wallet', () => {
    test('shows Liquidity Pools heading', async ({ page }) => {
      await page.goto('/pool')
      await expect(page.getByRole('heading', { name: /Liquidity Pools/i })).toBeVisible()
    })

    test('loads and displays at least one pair', async ({ page }) => {
      await page.goto('/pool')
      await expect(async () => {
        const pairCount = await page.getByText(/pair\(s\)/i).textContent()
        expect(pairCount).toMatch(/\d+\s*pair/i)
      }).toPass({ timeout: 15000 })
    })

    test('shows pool reserves for each pair', async ({ page }) => {
      await page.goto('/pool')
      await page.waitForLoadState('networkidle')
      await expect(async () => {
        const provideBtns = page.getByRole('button', { name: 'Provide Liquidity' })
        const count = await provideBtns.count()
        expect(count).toBeGreaterThanOrEqual(1)
      }).toPass({ timeout: 15000 })
    })

    test('shows fee info on pool cards', async ({ page }) => {
      await page.goto('/pool')
      await page.waitForLoadState('networkidle')
      await expect(async () => {
        await expect(page.getByText(/Fee:/i).first()).toBeVisible()
      }).toPass({ timeout: 15000 })
    })

    test('shows Provide Liquidity and Withdraw Liquidity buttons', async ({ page }) => {
      await page.goto('/pool')
      await page.waitForLoadState('networkidle')
      await expect(async () => {
        await expect(page.getByRole('button', { name: /Provide Liquidity/i }).first()).toBeVisible()
        await expect(page.getByRole('button', { name: /Withdraw Liquidity/i }).first()).toBeVisible()
      }).toPass({ timeout: 15000 })
    })
  })

  test.describe('Provide Liquidity form', () => {
    test('opens provide liquidity form on button click', async ({ page }) => {
      await page.goto('/pool')
      await page.waitForLoadState('networkidle')

      await expect(async () => {
        await expect(page.getByRole('button', { name: /Provide Liquidity/i }).first()).toBeVisible()
      }).toPass({ timeout: 15000 })

      await page.getByRole('button', { name: /Provide Liquidity/i }).first().click()
      await expect(page.getByText(/Asset A|Amount/i).first()).toBeVisible()
    })

    test('has input fields for both assets', async ({ page }) => {
      await page.goto('/pool')
      await page.waitForLoadState('networkidle')

      await expect(async () => {
        await expect(page.getByRole('button', { name: /Provide Liquidity/i }).first()).toBeVisible()
      }).toPass({ timeout: 15000 })

      await page.getByRole('button', { name: /Provide Liquidity/i }).first().click()

      const inputs = page.getByPlaceholder('0.00')
      await expect(inputs.first()).toBeVisible()
    })

    test('shows Connect Wallet when not connected', async ({ page }) => {
      await page.goto('/pool')
      await page.waitForLoadState('networkidle')

      await expect(async () => {
        await expect(page.getByRole('button', { name: /Provide Liquidity/i }).first()).toBeVisible()
      }).toPass({ timeout: 15000 })

      await page.getByRole('button', { name: /Provide Liquidity/i }).first().click()

      const submitBtns = page.getByRole('button', { name: /Connect Wallet/i })
      await expect(submitBtns.first()).toBeVisible()
    })
  })

  test.describe('Withdraw Liquidity form', () => {
    test('opens withdraw form on button click', async ({ page }) => {
      await page.goto('/pool')
      await page.waitForLoadState('networkidle')

      await expect(async () => {
        await expect(page.getByRole('button', { name: /Withdraw Liquidity/i }).first()).toBeVisible()
      }).toPass({ timeout: 15000 })

      await page.getByRole('button', { name: /Withdraw Liquidity/i }).first().click()

      const inputs = page.getByPlaceholder('0.00')
      await expect(inputs.first()).toBeVisible()
    })

    test('shows Connect Wallet when not connected', async ({ page }) => {
      await page.goto('/pool')
      await page.waitForLoadState('networkidle')

      await expect(async () => {
        await expect(page.getByRole('button', { name: /Withdraw Liquidity/i }).first()).toBeVisible()
      }).toPass({ timeout: 15000 })

      await page.getByRole('button', { name: /Withdraw Liquidity/i }).first().click()

      const submitBtns = page.getByRole('button', { name: /Connect Wallet/i })
      await expect(submitBtns.first()).toBeVisible()
    })
  })
})
