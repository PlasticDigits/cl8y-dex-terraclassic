import { test, expect } from './fixtures/dev-wallet'
import { DESKTOP_HEADER_NAV_ROW_LABELS, TABLET_COMPACT_HEADER_NAV_ROW_LABELS } from '../src/components/common/navItems'
import { clickDesktopMoreNavItem } from './helpers/desktop-more-nav'
import { headerConnectButton, headerConnectedWalletButton } from './helpers/wallet-ui'

test.describe('Tablet compact header nav (GitLab #136)', () => {
  for (const { width, height, label } of [
    { width: 773, height: 743, label: 'repro viewport' },
    { width: 768, height: 1024, label: 'iPad Mini' },
    { width: 820, height: 1180, label: 'iPad Air' },
    { width: 912, height: 1368, label: 'Surface Pro 7' },
  ] as const) {
    test(`Swap + More inline row has no horizontal overlap at ${label} (${width}px)`, async ({ page }) => {
      await page.setViewportSize({ width, height })
      await page.goto('/')
      await page.waitForLoadState('networkidle')

      const nav = page.locator('header.app-header-shell nav.app-desktop-nav')
      await expect(nav).toBeVisible()

      const boxes: { x: number; y: number; width: number; height: number }[] = []
      for (const name of TABLET_COMPACT_HEADER_NAV_ROW_LABELS) {
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
          `overlap between ${TABLET_COMPACT_HEADER_NAV_ROW_LABELS[i]} and ${TABLET_COMPACT_HEADER_NAV_ROW_LABELS[i + 1]}`
        ).toBeLessThanOrEqual(nextLeft + epsilon)
      }

      await nav.getByRole('button', { name: 'More' }).click()
      await expect(page.getByRole('menuitem', { name: 'Pool' })).toBeVisible()
      await expect(page.getByRole('menuitem', { name: 'Charts' })).toBeVisible()
    })
  }
})

test.describe('Full desktop header nav', () => {
  test('primary links plus More have no horizontal overlap at wide viewport', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 })
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
})

test.describe('Navigation', () => {
  test('loads the app with CL8Y DEX branding', async ({ page }) => {
    await page.goto('/')
    await expect(page).toHaveTitle(/CL8Y DEX/)
    await expect(page.getByRole('link', { name: 'CL8Y DEX' })).toBeVisible()
    await expect(page.getByText(/Terra Classic ecosystem/i)).toHaveCount(0)
  })

  test('shows persistent environment strip and NFA footer copy (GitLab #138)', async ({ page }) => {
    await page.goto('/')
    await expect(page.getByRole('status', { name: /Environment:/i })).toBeVisible()
    await expect(page.getByText(/Nothing here is financial/i)).toBeVisible()
  })

  test('shows NFA footer copy promptly after route changes (GitLab #138)', async ({ page }) => {
    const nfa = page.getByText(/Nothing here is financial/i)
    await page.goto('/')
    await expect(nfa).toBeVisible()

    await page.getByRole('link', { name: 'Pool' }).click()
    await expect(page).toHaveURL(/\/pool/)
    await expect(page.getByRole('heading', { name: /Liquidity Pools/i })).toBeVisible()
    await expect(nfa).toBeVisible({ timeout: 3_000 })

    await clickDesktopMoreNavItem(page, 'Fee Tiers')
    await expect(page).toHaveURL(/\/tiers/)
    await expect(nfa).toBeVisible({ timeout: 3_000 })

    await page.getByRole('link', { name: 'Trade' }).click()
    await expect(page).toHaveURL(/\/trade/)
    await expect(page.getByRole('heading', { name: 'Trade', exact: true })).toBeVisible()
    await expect(nfa).toBeVisible({ timeout: 3_000 })
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

  test('desktop primary tabs change URL without reload (GitLab #182)', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await page.goto('/')
    await page.waitForLoadState('domcontentloaded')

    const desktopNav = page.locator('header.app-header-shell nav.app-desktop-nav')
    await expect(desktopNav.getByRole('link', { name: 'Pool' })).toBeVisible()

    await desktopNav.getByRole('link', { name: 'Pool' }).click()
    await expect(page).toHaveURL(/\/pool/)

    await desktopNav.getByRole('link', { name: 'Trade' }).click()
    await expect(page).toHaveURL(/\/trade/)

    await desktopNav.getByRole('link', { name: 'Charts' }).click()
    await expect(page).toHaveURL(/\/charts/)

    await desktopNav.getByRole('link', { name: 'Swap' }).click()
    await expect(page).toHaveURL(/\/(\?.*)?$/)
  })

  test('navigates to Trade page', async ({ page }) => {
    await page.goto('/')
    await page.getByRole('link', { name: 'Trade' }).click()
    await expect(page).toHaveURL(/\/trade/)
    await expect(page.getByRole('heading', { name: 'Trade', exact: true })).toBeVisible()
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

  test('theme toggle lives in sticky header on desktop (GitLab #170)', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 })
    await page.goto('/pool')
    await page.waitForLoadState('networkidle')

    const header = page.locator('header.app-header-shell')
    const themeGroup = header.locator('.app-header-theme-group')
    await expect(themeGroup).toBeVisible()
    await expect(themeGroup.getByRole('button', { name: 'Dark' })).toBeVisible()
    await expect(page.locator('footer .app-footer-theme-group')).toHaveCount(0)

    const stickyTop = await page.locator('.app-top-sticky').boundingBox()
    const themeBox = await themeGroup.boundingBox()
    expect(stickyTop).not.toBeNull()
    expect(themeBox).not.toBeNull()
    expect(themeBox!.y + themeBox!.height).toBeLessThanOrEqual(stickyTop!.y + stickyTop!.height + 2)

    await themeGroup.getByRole('button', { name: 'Light' }).click()
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')
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

test.describe('Connected wallet chip network (GitLab #186)', () => {
  test('shows network shortLabel on desktop trigger at 1280px', async ({ page, connectWallet }) => {
    await page.setViewportSize({ width: 1280, height: 800 })
    await connectWallet
    const wallet = headerConnectedWalletButton(page)
    await expect(wallet.getByTestId('wallet-network-short-label')).toBeVisible()
    await expect(wallet.getByTestId('wallet-network-short-label')).toHaveText('Local')
  })

  test('mobile chip keeps LUNC on trigger and hides network text label', async ({ page, connectWallet }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await connectWallet
    await expect(page.getByTestId('wallet-lunc-balance').first()).toBeVisible()
    await expect(headerConnectedWalletButton(page).getByTestId('wallet-network-short-label')).toBeHidden()
  })

  test('connected wallet chip does not overlap header More at tablet width', async ({ page, connectWallet }) => {
    await page.setViewportSize({ width: 773, height: 743 })
    await connectWallet
    const nav = page.locator('header.app-header-shell nav.app-desktop-nav')
    const more = nav.getByRole('button', { name: 'More' })
    const wallet = headerConnectedWalletButton(page)
    const moreBox = await more.boundingBox()
    const walletBox = await wallet.boundingBox()
    expect(moreBox, 'More button box').toBeTruthy()
    expect(walletBox, 'wallet chip box').toBeTruthy()
    expect(moreBox!.x + moreBox!.width).toBeLessThanOrEqual(walletBox!.x + 2)
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

  test('connected dropdown: copy, explorer, switch wallet (GitLab #185)', async ({ page, connectWallet }) => {
    await connectWallet
    await headerConnectedWalletButton(page).click()
    await expect(page.getByTestId('wallet-menu-copy-address')).toBeVisible()
    await expect(page.getByRole('menuitem', { name: 'View on explorer' })).toBeVisible()
    await page.getByRole('menuitem', { name: 'Switch wallet' }).click()
    await expect(page.getByRole('heading', { name: /Connect Wallet/i })).toBeVisible()
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
