import { test, expect } from './fixtures/dev-wallet'
import { DESKTOP_HEADER_NAV_ROW_LABELS } from '../src/components/common/navItems'
import { clickDesktopMoreNavItem } from './helpers/desktop-more-nav'
import { headerConnectButton, headerConnectedWalletButton } from './helpers/wallet-ui'

test.describe('Tablet header nav', () => {
  for (const { width, height, label } of [
    { width: 768, height: 1024, label: 'iPad Mini' },
    { width: 820, height: 1180, label: 'iPad Air' },
    { width: 912, height: 1368, label: 'Surface Pro 7' },
  ] as const) {
    test(`desktop primary nav has no horizontal overlap at ${label} (${width}px)`, async ({ page }) => {
      await page.setViewportSize({ width, height })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const nav = page.locator('header.app-header-shell nav.app-desktop-nav')
      await expect(nav).toBeVisible()

      const boxes: { x: number; y: number; width: number; height: number }[] = []
      for (const name of DESKTOP_HEADER_NAV_ROW_LABELS) {
        const loc = name === 'More' ? nav.getByRole('button', { name: 'More' }) : nav.getByRole('link', { name })
        const b = await loc.boundingBox()
        expect(b, `bounding box for ${name}`).toBeTruthy()
        boxes.push(b!)
      }

      const epsilon = 2
      for (let i = 0; i < boxes.length - 1; i++) {
        const right = boxes[i].x + boxes[i].width
        const nextLeft = boxes[i + 1].x
        expect(
          right,
          `overlap between ${DESKTOP_HEADER_NAV_ROW_LABELS[i]} and ${DESKTOP_HEADER_NAV_ROW_LABELS[i + 1]}`
        ).toBeLessThanOrEqual(nextLeft + epsilon)
      }
    })
  }
})

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
    await clickDesktopMoreNavItem(page, 'Fee Tiers')
    await expect(page).toHaveURL(/\/tiers/)
    await expect(page.getByRole('heading', { name: /Fee Discount Tiers/i })).toBeVisible()
  })

  test('navigates to Create Pair page', async ({ page }) => {
    await page.goto('/')
    await clickDesktopMoreNavItem(page, 'Create Pair')
    await expect(page).toHaveURL(/\/create/)
    await expect(page.getByRole('heading', { name: /Create Trading Pair/i })).toBeVisible()
  })

  test('footer shows Terra Classic branding', async ({ page }) => {
    await page.goto('/')
    await expect(page.getByText(/CL8Y DEX.*Terra Classic/i)).toBeVisible()
  })
})

test.describe('Mobile navigation', () => {
  test('More sheet keeps theme controls above the bottom nav', async ({ page }) => {
    await page.setViewportSize({ width: 430, height: 932 })
    await page.goto('/')

    await page.getByRole('button', { name: 'More' }).click()
    const sheet = page.getByRole('region', { name: 'More pages' })
    await expect(sheet).toBeVisible()

    const darkTheme = page.getByRole('button', { name: 'Dark theme' })
    await darkTheme.scrollIntoViewIfNeeded()
    await expect(darkTheme).toBeVisible()

    const darkBox = await darkTheme.boundingBox()
    const navBox = await page.locator('nav.app-mobile-nav-shell').boundingBox()
    expect(darkBox).not.toBeNull()
    expect(navBox).not.toBeNull()
    const darkBottom = darkBox!.y + darkBox!.height
    expect(darkBottom).toBeLessThanOrEqual(navBox!.y + 1)
  })

  test('More sheet theme controls clear bottom nav on short viewports (Surface Duo–like)', async ({ page }) => {
    await page.setViewportSize({ width: 540, height: 720 })
    await page.goto('/')

    await page.getByRole('button', { name: 'More' }).click()
    const darkTheme = page.getByRole('button', { name: 'Dark theme' })
    await darkTheme.scrollIntoViewIfNeeded()
    await expect(darkTheme).toBeVisible()

    const darkBox = await darkTheme.boundingBox()
    const navBox = await page.locator('nav.app-mobile-nav-shell').boundingBox()
    expect(darkBox).not.toBeNull()
    expect(navBox).not.toBeNull()
    expect(darkBox!.y + darkBox!.height).toBeLessThanOrEqual(navBox!.y + 1)
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
