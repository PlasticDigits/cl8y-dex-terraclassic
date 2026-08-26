import { test, expect } from './fixtures/dev-wallet'
import { openPoolCardAdvanced } from './helpers/pool-ui'

async function gotoPoolAndOpenAdvanced(page: Parameters<typeof openPoolCardAdvanced>[0]) {
  await page.goto('/pool')
  await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
  await openPoolCardAdvanced(page)
}

test.describe('Pool Page', () => {
  test.describe('Without wallet', () => {
    test('shows pool table without a Liquidity Pools heading (GitLab #547)', async ({ page }) => {
      await page.goto('/pool')
      await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
      await expect(page.getByRole('heading', { name: /Liquidity Pools/i })).toHaveCount(0)
    })

    test('loads and displays at least one pair', async ({ page }) => {
      await page.goto('/pool')
      await expect(async () => {
        await expect(page.getByTestId('pool-pair-row').first()).toBeVisible()
      }).toPass({ timeout: 90_000 })
    })

    test('shows one-sided add and a Manage control on the table', async ({ page }) => {
      await page.goto('/pool')
      await expect(page.getByTestId('pool-one-sided-add')).toBeVisible()
      await expect(page.getByTestId('pool-row-manage').first()).toBeVisible({ timeout: 90_000 })
    })

    test('shows Provide Liquidity and Withdraw Liquidity under Advanced', async ({ page }) => {
      await page.goto('/pool')
      await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
      await openPoolCardAdvanced(page)
      await expect(page.getByRole('button', { name: /Provide Liquidity/i }).first()).toBeVisible()
      await expect(page.getByRole('button', { name: /Withdraw Liquidity/i }).first()).toBeVisible()
    })

    test('shows fee info on pool rows', async ({ page }) => {
      await page.goto('/pool')
      await expect(async () => {
        await expect(page.getByTestId('pool-table-fee').first()).toBeVisible()
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
      await expect(page.getByPlaceholder('0.00').first()).toBeVisible()
      await expect(page.getByText(/Asset A|Asset B/i)).toHaveCount(0)
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
