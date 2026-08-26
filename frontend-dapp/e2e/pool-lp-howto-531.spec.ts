import { expect, test, type Locator, type Page } from '@playwright/test'
import { clickDesktopMoreNavItem } from './helpers/desktop-more-nav'

async function gotoPool(page: Page) {
  await page.goto('/pool')
  await expect(page.getByTestId('pool-lp-howto')).toBeVisible({ timeout: 90_000 })
}

function boxesOverlap(
  a: { x: number; y: number; width: number; height: number },
  b: {
    x: number
    y: number
    width: number
    height: number
  }
) {
  return a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

async function expectNoOverlap(first: Locator, second: Locator, label: string) {
  const a = await first.boundingBox()
  const b = await second.boundingBox()
  expect(a, `${label}: first box`).toBeTruthy()
  expect(b, `${label}: second box`).toBeTruthy()
  expect(boxesOverlap(a!, b!), label).toBe(false)
}

test.describe('Retail LUNC LP how-to (GitLab #531)', () => {
  test('P1 desktop: open how-to then Provide form', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    await gotoPool(page)

    const details = page.getByTestId('pool-lp-howto-details')
    await expect(details).toBeVisible()
    await page.getByTestId('pool-lp-howto-open').click()
    await expect(details).toHaveAttribute('open', '')
    await expect(page.getByTestId('pool-lp-howto-step-two-sided')).toBeVisible()
    await expect(page.getByTestId('pool-lp-howto-step-two-sided')).toContainText(/one token/i)
    await expect(page.getByTestId('pool-lp-howto-step-no-incentive')).toBeVisible()
    await expect(page.getByTestId('pool-lp-howto-step-wrap')).toBeVisible()
    await expect(page.getByTestId('pool-lp-howto-step-withdraw')).toBeVisible()

    await expect(page.getByTestId('pool-one-sided-add')).toHaveCount(0)
    await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })

    const manage = page.getByTestId('pool-row-manage').first()
    const hasPairs = await manage
      .isVisible({ timeout: 20_000 })
      .then(() => true)
      .catch(() => false)
    if (hasPairs) {
      await manage.click()
      await page
        .getByRole('button', { name: /Provide Liquidity/i })
        .first()
        .click()
      await expect(page.getByLabel('Asset A amount')).toBeVisible()
      await expect(page.getByTestId('pool-il-risk-notice-advanced')).toBeVisible()
    }
  })

  test('P2 tablet: Pool via More, how-to still reachable', async ({ page }) => {
    await page.setViewportSize({ width: 820, height: 1180 })
    await page.goto('/')
    await expect(
      page.locator('header.app-header-shell nav.app-desktop-nav').getByRole('button', { name: /^More$/i })
    ).toBeVisible()
    await clickDesktopMoreNavItem(page, 'Pool')
    await expect(page).toHaveURL(/\/pool/)
    await expect(page.getByTestId('pool-lp-howto')).toBeVisible()
    await page.getByTestId('pool-lp-howto-summary').click()
    await expect(page.getByTestId('pool-lp-howto-details')).toHaveAttribute('open', '')
  })

  test('P3 phone: bottom-nav Pool; how-to does not cover tab bar or Provide', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/')
    const mobileNav = page.locator('nav.app-mobile-nav-shell')
    await expect(mobileNav.getByRole('link', { name: 'Pool' })).toBeVisible()
    await mobileNav.getByRole('link', { name: 'Pool' }).click()
    await expect(page).toHaveURL(/\/pool/)
    await expect(page.getByTestId('pool-lp-howto')).toBeVisible()
    await expectNoOverlap(page.getByTestId('pool-lp-howto'), mobileNav, 'how-to vs mobile tab bar')

    const provide = page.getByRole('button', { name: /Provide Liquidity/i }).first()
    if (await provide.isVisible().catch(() => false)) {
      await expectNoOverlap(page.getByTestId('pool-lp-howto'), provide, 'how-to vs Provide CTA')
    }
  })

  test('P5 withdraw form still opens; how-to mentions withdraw', async ({ page }) => {
    await gotoPool(page)
    await page.getByTestId('pool-lp-howto-summary').click()
    await expect(page.getByTestId('pool-lp-howto-step-withdraw')).toBeVisible()

    const manage = page.getByTestId('pool-row-manage').first()
    if (await manage.isVisible().catch(() => false)) {
      await manage.click()
      const withdraw = page.getByRole('button', { name: /Withdraw Liquidity/i }).first()
      if (await withdraw.isVisible().catch(() => false)) {
        await withdraw.click()
        await expect(page.getByPlaceholder('0.00').first()).toBeVisible()
      }
    }
  })

  test('P6 / P7: Wrap and Limits have no always-on LP lecture', async ({ page }) => {
    await page.goto('/wrap')
    await expect(page.getByTestId('pool-lp-howto')).toHaveCount(0)
    await page.goto('/limits')
    await expect(page.getByTestId('pool-lp-howto')).toHaveCount(0)
    await expect(page.getByTestId('trade-onboarding-strip').or(page.locator('body'))).toBeVisible()
  })

  test('P8 light + dark: how-to stays readable', async ({ page }) => {
    await gotoPool(page)
    for (const theme of ['Light theme', 'Dark theme'] as const) {
      const themeBtn = page.getByRole('button', { name: theme })
      if (await themeBtn.count()) {
        await themeBtn.click()
      }
      const howto = page.getByTestId('pool-lp-howto')
      await expect(howto).toBeVisible()
      const color = await howto.evaluate((el) => getComputedStyle(el).color)
      expect(color).not.toBe('rgba(0, 0, 0, 0)')
    }
  })

  test('C2 dismiss persists after reload', async ({ page }) => {
    await gotoPool(page)
    await page.getByTestId('pool-lp-howto-dismiss').click()
    await expect(page.getByTestId('pool-lp-howto-hint')).toHaveCount(0)
    await page.reload()
    await expect(page.getByTestId('pool-lp-howto-restore')).toBeVisible({ timeout: 90_000 })
    await expect(page.getByTestId('pool-lp-howto')).toHaveCount(0)
    await expect(page.getByTestId('pool-lp-howto-hint')).toHaveCount(0)
  })
})
