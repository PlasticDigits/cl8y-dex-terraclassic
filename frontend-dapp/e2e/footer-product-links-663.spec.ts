import type { Page } from '@playwright/test'
import { test, expect } from './fixtures/dev-wallet'
import { clickDesktopMoreNavItem } from './helpers/desktop-more-nav'

const HOME_HREF = 'https://cl8y.com/'
const BRIDGE_HREF = 'https://bridge.cl8y.com/'

async function expectFooterProductLinkAttrs(page: Page, testId: string, href: string) {
  const link = page.locator('footer.app-footer-shell').getByTestId(testId)
  await expect(link).toBeVisible()
  await expect(link).toHaveAttribute('href', href)
  await expect(link).toHaveAttribute('target', '_blank')
  const rel = await link.getAttribute('rel')
  expect(rel, `${testId} rel`).toBeTruthy()
  expect(rel).toContain('noopener')
  expect(rel).toContain('noreferrer')
}

test.describe('Footer official CL8Y product links (GitLab #663)', () => {
  test('desktop 1280 shows Homepage and Bridge in the footer shell', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 })
    await page.goto('/')
    await page.waitForLoadState('domcontentloaded')

    const footer = page.locator('footer.app-footer-shell')
    await expect(footer).toBeVisible()
    await expectFooterProductLinkAttrs(page, 'footer-product-home', HOME_HREF)
    await expectFooterProductLinkAttrs(page, 'footer-product-bridge', BRIDGE_HREF)
    await expect(footer.getByRole('navigation', { name: 'CL8Y products' })).toBeVisible()
    await expect(footer.locator('iframe')).toHaveCount(0)
  })

  test('phone 375 product links wrap above the mobile tab bar', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 })
    await page.goto('/')
    await page.waitForLoadState('domcontentloaded')

    const home = page.locator('footer.app-footer-shell').getByTestId('footer-product-home')
    const bridge = page.locator('footer.app-footer-shell').getByTestId('footer-product-bridge')
    await home.scrollIntoViewIfNeeded()
    await expect(home).toBeVisible()
    await expect(bridge).toBeVisible()

    const homeBox = await home.boundingBox()
    const bridgeBox = await bridge.boundingBox()
    const navBox = await page.locator('nav.app-mobile-nav-shell').boundingBox()
    expect(homeBox, 'homepage box').toBeTruthy()
    expect(bridgeBox, 'bridge box').toBeTruthy()
    expect(navBox, 'mobile nav box').toBeTruthy()
    expect(homeBox!.y + homeBox!.height).toBeLessThanOrEqual(navBox!.y + 1)
    expect(bridgeBox!.y + bridgeBox!.height).toBeLessThanOrEqual(navBox!.y + 1)
  })

  test('product links and NFA remain after Swap → Pool → Trade', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 })
    await page.goto('/')
    const nfa = page.getByText(/Nothing here is financial/i)
    await expect(nfa).toBeVisible()
    await expectFooterProductLinkAttrs(page, 'footer-product-home', HOME_HREF)
    await expectFooterProductLinkAttrs(page, 'footer-product-bridge', BRIDGE_HREF)

    await page.getByRole('link', { name: 'Pool' }).click()
    await expect(page).toHaveURL(/\/pool/)
    await expect(nfa).toBeVisible({ timeout: 3_000 })
    await expect(page.getByTestId('footer-product-home')).toBeVisible()
    await expect(page.getByTestId('footer-product-bridge')).toBeVisible()
    await expect(page.getByTestId('security-posture-doc-link')).toBeVisible()

    await page.getByRole('link', { name: 'Trade' }).click()
    await expect(page).toHaveURL(/\/trade/)
    await expect(nfa).toBeVisible({ timeout: 3_000 })
    await expect(page.getByTestId('footer-product-home')).toBeVisible()
    await expect(page.getByTestId('footer-product-bridge')).toBeVisible()
  })

  test('header brand has no ecosystem kicker; More lists Protocol not Bridge', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 })
    await page.goto('/')
    await expect(page.getByRole('link', { name: 'CL8Y DEX' })).toBeVisible()
    await expect(page.getByText(/Terra Classic ecosystem/i)).toHaveCount(0)

    await page
      .locator('header.app-header-shell nav.app-desktop-nav')
      .getByRole('button', { name: /^More$/i })
      .click()
    const menu = page.locator('header.app-header-shell .app-menu')
    await expect(menu.getByRole('menuitem', { name: 'Protocol' })).toBeVisible()
    await expect(menu.getByRole('menuitem', { name: 'Bridge' })).toHaveCount(0)
    await expect(menu.getByRole('menuitem', { name: 'Homepage' })).toHaveCount(0)
  })

  test('product links stay visible after toggling light theme', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 })
    await page.goto('/')
    const themeGroup = page.locator('header.app-header-shell .app-header-theme-group')
    await themeGroup.getByRole('button', { name: 'Light theme' }).click()
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')
    await expect(page.getByTestId('footer-product-home')).toBeVisible()
    await expect(page.getByTestId('footer-product-bridge')).toBeVisible()

    await themeGroup.getByRole('button', { name: 'Dark theme' }).click()
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
    await expect(page.getByTestId('footer-product-home')).toBeVisible()
    await expect(page.getByTestId('footer-product-bridge')).toBeVisible()
  })

  test('mobile More sheet stays in-app and does not list Bridge', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 })
    await page.goto('/')
    await page.getByRole('button', { name: 'More' }).click()
    const sheet = page.getByRole('region', { name: 'More pages' })
    await expect(sheet).toBeVisible()
    await expect(sheet.getByRole('link', { name: 'Protocol' })).toBeVisible()
    await expect(sheet.getByRole('link', { name: 'Bridge' })).toHaveCount(0)
    await expect(page.locator('footer.app-footer-shell').getByTestId('footer-product-bridge')).toBeVisible()
  })

  test('More still reaches Protocol (in-app) without leaving the DEX', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 })
    await page.goto('/')
    await clickDesktopMoreNavItem(page, 'Protocol')
    await expect(page).toHaveURL(/\/protocol/)
    await expect(page.getByTestId('footer-product-home')).toBeVisible()
  })
})
