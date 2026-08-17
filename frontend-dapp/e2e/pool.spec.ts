import { test, expect } from './fixtures/dev-wallet'
import { openPoolCardAdvanced } from './helpers/pool-ui'

async function gotoPoolAndOpenAdvanced(page: Parameters<typeof openPoolCardAdvanced>[0]) {
  await page.goto('/pool')
  await expect(async () => {
    await expect(page.getByTestId('pool-card-advanced').first()).toBeVisible()
  }).toPass({ timeout: 90_000 })
  await openPoolCardAdvanced(page)
}

test.describe('Pool Page', () => {
  test.describe('Without wallet', () => {
    test('shows Liquidity Pools heading', async ({ page }) => {
      await page.goto('/pool')
      await expect(page.getByRole('heading', { name: /Liquidity Pools/i })).toBeVisible()
    })

    test('loads and displays at least one pair', async ({ page }) => {
      await page.goto('/pool')
      await expect(async () => {
        const pairCount = await page.getByText(/pair\(s\) \(indexer total\)/i).textContent()
        expect(pairCount).toMatch(/[\d,]+\s*pair/i)
        const m = pairCount?.match(/([\d,]+)\s*pair/i)
        expect(m).toBeTruthy()
        const n = parseInt(m![1].replace(/,/g, ''), 10)
        expect(n).toBeGreaterThan(0)
      }).toPass({ timeout: 90_000 })
    })

    test('shows one-sided add and Advanced on pool cards', async ({ page }) => {
      await page.goto('/pool')
      await expect(page.getByTestId('pool-one-sided-add')).toBeVisible()
      await expect(async () => {
        await expect(page.getByTestId('pool-card-advanced').first()).toBeVisible()
      }).toPass({ timeout: 90_000 })
    })

    test('shows Provide Liquidity and Withdraw Liquidity under Advanced', async ({ page }) => {
      await page.goto('/pool')
      await expect(async () => {
        await expect(page.getByTestId('pool-card-advanced').first()).toBeVisible()
      }).toPass({ timeout: 90_000 })
      await openPoolCardAdvanced(page)
      await expect(page.getByRole('button', { name: /Provide Liquidity/i }).first()).toBeVisible()
      await expect(page.getByRole('button', { name: /Withdraw Liquidity/i }).first()).toBeVisible()
    })

    test('shows fee info on pool cards', async ({ page }) => {
      await page.goto('/pool')
      await expect(async () => {
        await expect(page.getByText(/Fee:/i).first()).toBeVisible()
      }).toPass({ timeout: 90_000 })
    })

    test('shows Provide Liquidity and Withdraw Liquidity buttons', async ({ page }) => {
      await gotoPoolAndOpenAdvanced(page)
      await expect(page.getByRole('button', { name: /Provide Liquidity/i }).first()).toBeVisible()
      await expect(page.getByRole('button', { name: /Withdraw Liquidity/i }).first()).toBeVisible()
    })
  })

  test.describe('Provide Liquidity form', () => {
    test('opens provide liquidity form on button click', async ({ page }) => {
      await gotoPoolAndOpenAdvanced(page)

      await page
        .getByRole('button', { name: /Provide Liquidity/i })
        .first()
        .click()
      await expect(page.getByText(/Asset A|Amount/i).first()).toBeVisible()
    })

    test('has input fields for both assets', async ({ page }) => {
      await gotoPoolAndOpenAdvanced(page)

      await page
        .getByRole('button', { name: /Provide Liquidity/i })
        .first()
        .click()

      const inputs = page.getByPlaceholder('0.00')
      await expect(inputs.first()).toBeVisible()
    })

    test('shows Connect Wallet when not connected', async ({ page }) => {
      await gotoPoolAndOpenAdvanced(page)

      await page
        .getByRole('button', { name: /Provide Liquidity/i })
        .first()
        .click()

      const submitBtns = page.getByRole('button', { name: /Connect Wallet/i })
      await expect(submitBtns.first()).toBeVisible()
    })
  })

  test.describe('Provide Liquidity (connected)', () => {
    test('shows per-asset Balance row in add-LP panel', async ({ page, connectWallet }) => {
      await connectWallet
      await gotoPoolAndOpenAdvanced(page)

      await page
        .getByRole('button', { name: /Provide Liquidity/i })
        .first()
        .click()

      const balRows = page.getByText(/^Balance:/i)
      await expect(balRows.first()).toBeVisible()
      await expect(balRows.nth(1)).toBeVisible()
    })
  })

  test.describe('Withdraw Liquidity form', () => {
    test('opens withdraw form on button click', async ({ page }) => {
      await gotoPoolAndOpenAdvanced(page)

      await page
        .getByRole('button', { name: /Withdraw Liquidity/i })
        .first()
        .click()

      const inputs = page.getByPlaceholder('0.00')
      await expect(inputs.first()).toBeVisible()
    })

    test('shows Connect Wallet when not connected', async ({ page }) => {
      await gotoPoolAndOpenAdvanced(page)

      await page
        .getByRole('button', { name: /Withdraw Liquidity/i })
        .first()
        .click()

      const submitBtns = page.getByRole('button', { name: /Connect Wallet/i })
      await expect(submitBtns.first()).toBeVisible()
    })
  })
})
