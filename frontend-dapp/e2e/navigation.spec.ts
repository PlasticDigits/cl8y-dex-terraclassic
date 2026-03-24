import { test, expect } from './fixtures/dev-wallet'
import { headerConnectButton, headerConnectedWalletButton } from './helpers/wallet-ui'

test.describe('Navigation', () => {
  test('loads the app with CL8Y DEX branding', async ({ page }) => {
    await page.goto('/')
    await expect(page).toHaveTitle(/CL8Y DEX/)
    await expect(page.getByRole('link', { name: 'CL8Y DEX' })).toBeVisible()
  })

  test('navigates to Swap page by default', async ({ page }) => {
    await page.goto('/')
    await expect(page.getByRole('heading', { name: 'Swap' })).toBeVisible()
  })

  test('navigates to Pool page', async ({ page }) => {
    await page.goto('/')
    await page.getByRole('link', { name: 'Pool' }).click()
    await expect(page).toHaveURL(/\/pool/)
    await expect(page.getByRole('heading', { name: /Liquidity Pools/i })).toBeVisible()
  })

  test('navigates to Fee Tiers page', async ({ page }) => {
    await page.goto('/')
    await page.getByRole('link', { name: 'Fee Tiers' }).click()
    await expect(page).toHaveURL(/\/tiers/)
    await expect(page.getByRole('heading', { name: /Fee Discount Tiers/i })).toBeVisible()
  })

  test('navigates to Create Pair page', async ({ page }) => {
    await page.goto('/')
    await page.getByRole('link', { name: 'Create Pair' }).click()
    await expect(page).toHaveURL(/\/create/)
    await expect(page.getByRole('heading', { name: /Create Trading Pair/i })).toBeVisible()
  })

  test('footer shows Terra Classic branding', async ({ page }) => {
    await page.goto('/')
    await expect(page.getByText(/CL8Y DEX.*Terra Classic/i)).toBeVisible()
  })
})

test.describe('Wallet Connection', () => {
  test('shows connect control in header when disconnected', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')
    await expect(headerConnectButton(page)).toBeVisible()
  })

  test('opens wallet modal on click', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')
    await headerConnectButton(page).click()
    await expect(page.getByRole('heading', { name: /Connect Wallet/i })).toBeVisible()
    await expect(page.getByRole('button', { name: /Simulated Wallet/i })).toBeVisible()
  })

  test('connects simulated dev wallet', async ({ page, connectWallet }) => {
    await connectWallet
    await expect(headerConnectedWalletButton(page)).toBeVisible()
  })

  test('disconnects wallet', async ({ page, connectWallet }) => {
    await connectWallet
    await expect(headerConnectedWalletButton(page)).toBeVisible()
    await headerConnectedWalletButton(page).click()
    await page.getByRole('menuitem', { name: 'Disconnect' }).click()
    await expect(headerConnectButton(page)).toBeVisible()
  })

  test('wallet modal can be closed with X button', async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')
    await headerConnectButton(page).click()
    await expect(page.getByRole('heading', { name: /Connect Wallet/i })).toBeVisible()
    await page.getByRole('button', { name: /close modal/i }).click()
    await expect(page.getByRole('heading', { name: /Connect Wallet/i })).not.toBeVisible()
  })
})
