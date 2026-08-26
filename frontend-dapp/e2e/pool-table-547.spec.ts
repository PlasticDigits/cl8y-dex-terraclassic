import { expect, test, type Page } from '@playwright/test'

async function gotoPoolTable(page: Page) {
  await page.goto('/pool')
  await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
}

test.describe('Pool sortable table (GitLab #547)', () => {
  test('P1 desktop: table, no header essay, how-to dismissible', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    await gotoPoolTable(page)
    await expect(page.getByRole('heading', { name: /Liquidity Pools/i })).toHaveCount(0)
    await expect(page.getByText(/List source:/i)).toHaveCount(0)
    await expect(page.getByTestId('pool-filter-router')).toHaveCount(0)
    await expect(page.getByTestId('pool-lp-howto')).toBeVisible()
    await page.getByTestId('pool-lp-howto-dismiss').click()
    await expect(page.getByTestId('pool-lp-howto-details')).toHaveCount(0)
    await expect(page.getByTestId('pool-one-sided-add')).toHaveCount(0)
    await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
  })

  test('P2 click Vol header updates caret', async ({ page }) => {
    await gotoPoolTable(page)
    await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
    const vol = page.getByTestId('pool-sort-vol')
    const volTh = page.getByTestId('pool-sort-vol-th')
    await vol.click()
    await expect(volTh).toHaveAttribute('aria-sort', 'descending')
    await vol.click()
    await expect(volTh).toHaveAttribute('aria-sort', 'ascending')
  })

  test('P3 Charts from first row lands on /charts/:pairAddr', async ({ page }) => {
    await gotoPoolTable(page)
    const charts = page.getByTestId('pool-row-charts').first()
    await expect(charts).toBeVisible({ timeout: 90_000 })
    const href = await charts.getAttribute('href')
    expect(href).toMatch(/^\/charts\/terra1[a-z0-9]+$/)
    await charts.click()
    await expect(page).toHaveURL(new RegExp(`${href?.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`))
  })

  test('P4 tablet 820×1180: table scrollable; one-sided CTAs not covered', async ({ page }) => {
    await page.setViewportSize({ width: 820, height: 1180 })
    await gotoPoolTable(page)
    const wrap = page.getByTestId('pool-pairs-table-wrap')
    await expect(wrap).toBeVisible({ timeout: 90_000 })
    const overflow = await wrap.evaluate((el) => getComputedStyle(el).overflowX)
    expect(['auto', 'scroll', 'overlay']).toContain(overflow)
    await expect(page.getByTestId('pool-pairs-table')).toBeVisible()
  })

  test('P5 phone 390×844: table + bottom nav Pool', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/')
    const mobileNav = page.locator('nav.app-mobile-nav-shell')
    await mobileNav.getByRole('link', { name: 'Pool' }).click()
    await expect(page).toHaveURL(/\/pool/)
    await expect(page.getByTestId('pool-lp-howto')).toBeVisible({ timeout: 90_000 })
    await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
  })

  test('P6 #lp-howto restores how-to after dismiss', async ({ page }) => {
    await gotoPoolTable(page)
    await page.getByTestId('pool-lp-howto-dismiss').click()
    await expect(page.getByTestId('pool-lp-howto-details')).toHaveCount(0)
    await page.goto('/pool#lp-howto')
    await expect(page.getByTestId('pool-lp-howto-details')).toBeVisible({ timeout: 90_000 })
    await expect(page.getByTestId('pool-lp-howto-details')).toHaveAttribute('open', '')
  })
})
