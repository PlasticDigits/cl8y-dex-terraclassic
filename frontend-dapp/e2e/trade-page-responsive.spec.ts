import { test, expect } from '@playwright/test'

/**
 * Needs the same LocalTerra + LCD + indexer stack as other Playwright suites
 * (`frontend-dapp/.env.local`, global setup). Without a resolved `terra1…` pair,
 * `OrderBookPanel` has no `Order book` heading and these tests cannot run.
 */
test.describe('Trade page responsive layout (GitLab #146)', () => {
  test('tablet width places chart left of ticket on one row, order book below', async ({ page }) => {
    await page.setViewportSize({ width: 820, height: 1180 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')

    const workspace = page.getByTestId('trade-sub-lg-workspace')
    const priceHeading = workspace.getByRole('heading', { name: 'Price (USD)' })
    const placeLimitHeading = workspace.getByRole('heading', { name: 'Place limit' })
    const orderBookHeading = workspace.getByRole('heading', { name: 'Order book' })

    await expect(async () => {
      await expect(orderBookHeading).toBeVisible()
      await expect(priceHeading).toBeVisible()
      await expect(placeLimitHeading).toBeVisible()
    }).toPass({ timeout: 90_000 })

    const chartBox = await priceHeading.boundingBox()
    const ticketBox = await placeLimitHeading.boundingBox()
    const bookBox = await orderBookHeading.boundingBox()

    expect(chartBox, 'chart heading box').toBeTruthy()
    expect(ticketBox, 'ticket heading box').toBeTruthy()
    expect(bookBox, 'order book heading box').toBeTruthy()

    expect(chartBox!.x, 'chart should be left of ticket').toBeLessThan(ticketBox!.x)
    expect(Math.abs(chartBox!.y - ticketBox!.y), 'chart and ticket share a visual row').toBeLessThan(120)

    const rowBottom = Math.max(chartBox!.y + chartBox!.height, ticketBox!.y + ticketBox!.height)
    expect(bookBox!.y, 'order book sits below the chart/ticket row').toBeGreaterThanOrEqual(rowBottom - 48)
  })

  test('phone width stacks order book, ticket, then chart', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')

    const workspace = page.getByTestId('trade-sub-lg-workspace')
    const orderBookHeading = workspace.getByRole('heading', { name: 'Order book' })
    const placeLimitHeading = workspace.getByRole('heading', { name: 'Place limit' })
    const priceHeading = workspace.getByRole('heading', { name: 'Price (USD)' })

    await expect(async () => {
      await expect(orderBookHeading).toBeVisible()
      await expect(placeLimitHeading).toBeVisible()
      await expect(priceHeading).toBeVisible()
    }).toPass({ timeout: 90_000 })

    const bBook = await orderBookHeading.boundingBox()
    const bTicket = await placeLimitHeading.boundingBox()
    const bChart = await priceHeading.boundingBox()

    expect(bBook!.y).toBeLessThan(bTicket!.y)
    expect(bTicket!.y).toBeLessThan(bChart!.y)
  })
})
