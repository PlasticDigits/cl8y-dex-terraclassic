import { test, expect } from './fixtures/dev-wallet'
import { clickDesktopMoreNavItem } from './helpers/desktop-more-nav'
import {
  ARIA_SELECT_TOKEN_A,
  ARIA_SELECT_TOKEN_B,
  createPairTokenA,
  createPairTokenB,
  fillCreatePairCustom,
} from './helpers/create-pair-picker'

test.describe('Create Pair Page', () => {
  test('shows Create Trading Pair heading', async ({ page }) => {
    await page.goto('/create')
    await expect(page.getByRole('heading', { name: /Create Trading Pair/i })).toBeVisible()
  })

  test('has listed-token comboboxes for Token A and Token B', async ({ page }) => {
    await page.goto('/create')
    await expect(createPairTokenA(page)).toBeVisible()
    await expect(createPairTokenB(page)).toBeVisible()
    await expect(page.getByRole('combobox', { name: ARIA_SELECT_TOKEN_A })).toBeVisible()
    await expect(page.getByRole('combobox', { name: ARIA_SELECT_TOKEN_B })).toBeVisible()
  })

  test('shows labels for Token A and Token B', async ({ page }) => {
    await page.goto('/create')
    await expect(page.getByText(/Token A/i).first()).toBeVisible()
    await expect(page.getByText(/Token B/i).first()).toBeVisible()
  })

  test('shows prerequisites info box', async ({ page }) => {
    await page.goto('/create')
    await expect(page.getByText(/Before creating a pair/i)).toBeVisible()
    await expect(page.getByText(/valid CW20 contracts/i)).toBeVisible()
    await expect(page.getByText(/whitelisted by governance/i)).toBeVisible()
    await expect(page.getByText(/must not already exist/i)).toBeVisible()
  })

  test('shows Connect Wallet button when disconnected', async ({ page }) => {
    await page.goto('/create')
    const submitBtn = page.getByRole('button', { name: /Connect Wallet/i }).last()
    await expect(submitBtn).toBeVisible()
    await expect(submitBtn).toBeDisabled()
  })

  test('accepts terra address input in Token A custom field', async ({ page }) => {
    await page.goto('/create')
    const testAddr = 'terra1yyca08xqdgvjz0psg56z67ejh9xms6l436u8y58m82npdqqhmmtqzjqhh0'
    await fillCreatePairCustom(page, 'a', testAddr)
    await expect(page.getByLabel(/Token A Contract Address/i)).toHaveValue(testAddr)
  })

  test('accepts terra address input in Token B custom field', async ({ page }) => {
    await page.goto('/create')
    const testAddr = 'terra1yw4xvtc43me9scqfr2jr2gzvcxd3a9y4eq7gaukreugw2yd2f8tsrnr34u'
    await fillCreatePairCustom(page, 'b', testAddr)
    await expect(page.getByLabel(/Token B Contract Address/i)).toHaveValue(testAddr)
  })

  test.describe('With wallet connected', () => {
    test('shows Create Pair button when connected', async ({ page, connectWallet }) => {
      await connectWallet
      await clickDesktopMoreNavItem(page, 'Create Pair')
      await page.waitForURL('/create')

      const createBtn = page.getByRole('button', { name: /Create Pair/i })
      await expect(createBtn).toBeVisible()
    })

    test('Create Pair button is disabled without both addresses', async ({ page, connectWallet }) => {
      await connectWallet
      await clickDesktopMoreNavItem(page, 'Create Pair')
      await page.waitForURL('/create')

      const createBtn = page.getByRole('button', { name: /Create Pair/i })
      await expect(createBtn).toBeDisabled()
    })

    test('can fill both custom addresses and see enabled button', async ({ page, connectWallet }) => {
      await connectWallet
      await clickDesktopMoreNavItem(page, 'Create Pair')
      await page.waitForURL('/create')

      await fillCreatePairCustom(page, 'a', 'terra1yyca08xqdgvjz0psg56z67ejh9xms6l436u8y58m82npdqqhmmtqzjqhh0')
      await fillCreatePairCustom(page, 'b', 'terra1yw4xvtc43me9scqfr2jr2gzvcxd3a9y4eq7gaukreugw2yd2f8tsrnr34u')

      const createBtn = page.getByRole('button', { name: /Create Pair/i })
      await expect(createBtn).toBeEnabled()
    })
  })
})
