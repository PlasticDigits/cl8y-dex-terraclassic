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
    const chartCol = workspace.getByTestId('trade-sub-lg-chart-col')
    const ticketCol = workspace.getByTestId('trade-sub-lg-ticket-col')
    const orderBookHeading = workspace.getByRole('heading', { name: 'Order book' })

    await expect(async () => {
      await expect(chartCol).toBeVisible()
      await expect(ticketCol).toBeVisible()
      await expect(orderBookHeading).toBeVisible()
    }).toPass({ timeout: 90_000 })

    const chartBox = await chartCol.boundingBox()
    const ticketBox = await ticketCol.boundingBox()
    const bookBox = await orderBookHeading.boundingBox()

    expect(chartBox, 'chart column box').toBeTruthy()
    expect(ticketBox, 'ticket column box').toBeTruthy()
    expect(bookBox, 'order book heading box').toBeTruthy()

    expect(chartBox!.x, 'chart column should be left of ticket column').toBeLessThan(ticketBox!.x)
    expect(Math.abs(chartBox!.y - ticketBox!.y), 'chart and ticket columns share a grid row').toBeLessThan(8)

    const rowBottom = Math.max(chartBox!.y + chartBox!.height, ticketBox!.y + ticketBox!.height)
    expect(bookBox!.y, 'order book sits below the chart/ticket row').toBeGreaterThanOrEqual(rowBottom - 48)
  })

  test('desktop limit submit stays visible without scrolling the ticket (#348)', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')

    const workspace = page.getByTestId('trade-desktop-workspace')
    await expect(workspace).toBeVisible({ timeout: 90_000 })

    const submit = page.getByTestId('trade-limit-submit')
    await expect(submit).toBeVisible()
    const workspaceBox = await workspace.boundingBox()
    const box = await submit.boundingBox()
    expect(workspaceBox, 'workspace box').toBeTruthy()
    expect(box, 'submit button box').toBeTruthy()
    expect(box!.y + box!.height).toBeLessThanOrEqual(workspaceBox!.y + workspaceBox!.height + 2)
  })

  test('limit ticket footer is opaque; guards stay in flow; expiry can clear footer (#500 / #527)', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')

    const workspace = page.getByTestId('trade-desktop-workspace')
    await expect(workspace).toBeVisible({ timeout: 90_000 })

    const footer = page.getByTestId('trade-ticket-submit-footer')
    const guards = page.getByTestId('trade-limit-inline-guards')
    const scroll = page.getByTestId('trade-order-ticket-scroll')
    const submit = page.getByTestId('trade-limit-submit')
    await expect(footer).toBeAttached()
    await expect(guards).toBeAttached()
    await expect(scroll).toBeVisible()
    await expect(submit).toBeAttached()

    expect(
      await guards.evaluate((el) => el.closest('[data-testid="trade-ticket-submit-footer"]') == null),
      'place guards must not live inside the ticket footer'
    ).toBe(true)
    expect(
      await submit.evaluate((el) => Boolean(el.closest('[data-testid="trade-ticket-submit-footer"]'))),
      'Place limit button must live inside the ticket footer'
    ).toBe(true)
    expect(
      await footer.evaluate((el) => el.closest('[data-testid="trade-order-ticket-scroll"]') == null),
      'footer must be a sibling of the ticket scrollport, not inside it'
    ).toBe(true)

    await submit.scrollIntoViewIfNeeded()
    await page.waitForTimeout(50)

    const bgOpaque = await footer.evaluate((el) => {
      const s = getComputedStyle(el)
      const hasImage = s.backgroundImage !== 'none' && s.backgroundImage.length > 0
      const color = s.backgroundColor
      const transparent = color === 'rgba(0, 0, 0, 0)' || color === 'transparent'
      return hasImage || !transparent
    })
    expect(bgOpaque, 'ticket footer background must not be fully transparent').toBe(true)

    const hitFooter = await submit.evaluate((el) => {
      const r = el.getBoundingClientRect()
      const x = Math.min(Math.max(r.left + r.width / 2, 0), window.innerWidth - 1)
      const y = Math.min(Math.max(r.top + r.height / 2, 0), window.innerHeight - 1)
      const hit = document.elementFromPoint(x, y)
      return Boolean(hit?.closest('[data-testid="trade-ticket-submit-footer"]'))
    })
    expect(hitFooter, 'pointer hit on Place limit must land in the ticket footer').toBe(true)

    const expiry = page.locator('#trade-ticket-expiry-dt')
    await expect(expiry).toBeAttached()
    await expiry.evaluate((el) => {
      el.scrollIntoView({ block: 'center', inline: 'nearest' })
    })
    await page.waitForTimeout(50)
    const expiryBox = await expiry.boundingBox()
    const footerBox = await footer.boundingBox()
    expect(expiryBox, 'expiry input box').toBeTruthy()
    expect(footerBox, 'ticket footer box').toBeTruthy()
    expect(
      expiryBox!.y + expiryBox!.height,
      'expiry input should sit above the ticket footer after scroll-into-view'
    ).toBeLessThanOrEqual(footerBox!.y + 2)
  })

  test('phone width stacks order book, ticket, then chart', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')

    const workspace = page.getByTestId('trade-sub-lg-workspace')
    const orderBookHeading = workspace.getByRole('heading', { name: 'Order book' })
    const ticketHeading = workspace.getByTestId('trade-ticket-heading')
    const priceHeading = workspace.getByRole('heading', { name: 'Price (USD)' })

    await expect(async () => {
      await expect(orderBookHeading).toBeVisible()
      await expect(ticketHeading).toBeVisible()
      await expect(priceHeading).toBeVisible()
    }).toPass({ timeout: 90_000 })

    const bBook = await orderBookHeading.boundingBox()
    const bTicket = await ticketHeading.boundingBox()
    const bChart = await priceHeading.boundingBox()

    expect(bBook!.y).toBeLessThan(bTicket!.y)
    expect(bTicket!.y).toBeLessThan(bChart!.y)
  })
})

type Box = { x: number; y: number; width: number; height: number }

function boxesIntersect(a: Box, b: Box): boolean {
  return !(a.x + a.width <= b.x || b.x + b.width <= a.x || a.y + a.height <= b.y || b.y + b.height <= a.y)
}

function footerIsOpaque(el: Element): boolean {
  const s = getComputedStyle(el)
  const hasImage = s.backgroundImage !== 'none' && s.backgroundImage.length > 0
  const color = s.backgroundColor
  const transparent = color === 'rgba(0, 0, 0, 0)' || color === 'transparent'
  return hasImage || !transparent
}

test.describe('Trade ticket money-CTA dock (GitLab #527)', () => {
  test('P1 desktop Limit: submit docks to ticket card bottom and misses form controls', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')
    await expect(page.getByTestId('trade-desktop-workspace')).toBeVisible({ timeout: 90_000 })

    const card = page.getByTestId('trade-order-ticket-card')
    const submit = page.getByTestId('trade-limit-submit')
    const footer = page.getByTestId('trade-ticket-submit-footer')
    await expect(submit).toBeVisible()

    const cardBox = await card.boundingBox()
    const submitBox = await submit.boundingBox()
    expect(cardBox, 'ticket card').toBeTruthy()
    expect(submitBox, 'Place limit').toBeTruthy()
    expect(Math.abs(submitBox!.y + submitBox!.height - (cardBox!.y + cardBox!.height))).toBeLessThanOrEqual(8)

    const receive = page.getByTestId('limit-order-receive-field')
    const expiryChip = page.getByRole('button', { name: 'No expiry' }).first()
    const payChip = page.getByTestId('limit-order-escrow-frac-25')
    for (const loc of [receive, expiryChip]) {
      await expect(loc).toBeVisible()
      const box = await loc.boundingBox()
      expect(box, 'form control box').toBeTruthy()
      expect(boxesIntersect(submitBox!, box!), 'Place limit must not overlap Receive/Expiry').toBe(false)
    }
    if ((await payChip.count()) > 0) {
      const box = await payChip.boundingBox()
      if (box) {
        expect(boxesIntersect(submitBox!, box), 'Place limit must not overlap Pay % chips').toBe(false)
      }
    }

    expect(
      await submit.evaluate((el) => getComputedStyle(el).position !== 'fixed'),
      'A5: CTA must not be position:fixed'
    ).toBe(true)
    expect(
      await footer.evaluate((el) => Boolean(el.closest('[data-testid="trade-order-ticket-card"]'))),
      'A5: footer stays inside the ticket card'
    ).toBe(true)

    const expiryHit = await expiryChip.evaluate((el) => {
      const r = el.getBoundingClientRect()
      const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2)
      return Boolean(hit?.closest('[data-testid="trade-limit-submit"]'))
    })
    expect(expiryHit, 'A1: Expiry chip hit must not land on Place limit').toBe(false)
  })

  test('P2 desktop: footer stays docked after scrolling the ticket body', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')
    await expect(page.getByTestId('trade-desktop-workspace')).toBeVisible({ timeout: 90_000 })

    const scroll = page.getByTestId('trade-order-ticket-scroll')
    const card = page.getByTestId('trade-order-ticket-card')
    const submit = page.getByTestId('trade-limit-submit')
    await scroll.evaluate((el) => {
      el.scrollTop = el.scrollHeight
    })
    await page.waitForTimeout(50)
    const cardBox = await card.boundingBox()
    const submitBox = await submit.boundingBox()
    expect(Math.abs(submitBox!.y + submitBox!.height - (cardBox!.y + cardBox!.height))).toBeLessThanOrEqual(8)
  })

  test('P3 desktop Market: same footer dock; no leftover Place limit', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')
    await expect(page.getByTestId('trade-desktop-workspace')).toBeVisible({ timeout: 90_000 })

    await page.getByTestId('trade-order-tab-market').click()
    const market = page.getByTestId('trade-market-submit')
    await expect(market).toBeVisible()
    expect(await page.getByTestId('trade-limit-submit').count()).toBe(0)
    expect(await market.evaluate((el) => Boolean(el.closest('[data-testid="trade-ticket-submit-footer"]')))).toBe(true)

    const cardBox = await page.getByTestId('trade-order-ticket-card').boundingBox()
    const submitBox = await market.boundingBox()
    expect(Math.abs(submitBox!.y + submitBox!.height - (cardBox!.y + cardBox!.height))).toBeLessThanOrEqual(8)
  })

  test('P4 tablet: CTA docks in trade-sub-lg-ticket-col', async ({ page }) => {
    await page.setViewportSize({ width: 820, height: 1180 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')
    const ticketCol = page.getByTestId('trade-sub-lg-ticket-col')
    await expect(ticketCol).toBeVisible({ timeout: 90_000 })
    const submit = ticketCol.getByTestId('trade-limit-submit')
    await expect(submit).toBeVisible()
    const colBox = await ticketCol.boundingBox()
    const submitBox = await submit.boundingBox()
    expect(submitBox!.y + submitBox!.height).toBeLessThanOrEqual(colBox!.y + colBox!.height + 8)
  })

  test('P5 phone: CTA is not position:fixed and does not overlay expiry', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')
    await expect(page.getByTestId('trade-sub-lg-workspace')).toBeVisible({ timeout: 90_000 })

    const submit = page.getByTestId('trade-limit-submit')
    await expect(submit).toBeAttached()
    expect(await submit.evaluate((el) => getComputedStyle(el).position !== 'fixed')).toBe(true)

    const expiry = page.locator('#trade-ticket-expiry-dt')
    const expiryBox = await expiry.boundingBox()
    const submitBox = await submit.boundingBox()
    if (expiryBox && submitBox) {
      expect(boxesIntersect(expiryBox, submitBox), 'phone CTA must not overlay expiry').toBe(false)
    }
  })

  test('P6 / P7 expiry datetime scrollIntoView stays above footer', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')
    await expect(page.getByTestId('trade-desktop-workspace')).toBeVisible({ timeout: 90_000 })

    await page.locator('summary', { hasText: 'Advanced' }).first().click()
    const expiry = page.locator('#trade-ticket-expiry-dt')
    await expiry.evaluate((el) => el.scrollIntoView({ block: 'center', inline: 'nearest' }))
    await page.waitForTimeout(50)
    const footer = page.getByTestId('trade-ticket-submit-footer')
    const expiryBox = await expiry.boundingBox()
    const footerBox = await footer.boundingBox()
    expect(expiryBox!.y + expiryBox!.height).toBeLessThanOrEqual(footerBox!.y + 2)

    const hitFooter = await expiry.evaluate((el) => {
      const r = el.getBoundingClientRect()
      const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2)
      return Boolean(hit?.closest('[data-testid="trade-ticket-submit-footer"]'))
    })
    expect(hitFooter, 'P6: datetime hit must not land on the footer').toBe(false)
  })

  test('P10 no resize handles; hiding ticket expands the chart (GitLab #561)', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')
    await expect(page.getByTestId('trade-desktop-workspace')).toBeVisible({ timeout: 90_000 })

    await expect(page.getByTestId('trade-ticket-resize-handle')).toHaveCount(0)
    await expect(page.getByTestId('trade-book-chart-resize-handle')).toHaveCount(0)
    await expect(page.getByTestId('trade-chart-tape-resize-handle')).toHaveCount(0)
    expect(await page.locator('[data-panel-resize-handle-id]').count()).toBe(0)

    const chart = page.getByTestId('trade-desktop-chart-col')
    const ticketCol = page.getByTestId('trade-desktop-ticket-col')
    await expect(chart).toBeVisible()
    await expect(ticketCol).toBeVisible()
    const before = await chart.boundingBox()
    expect(before).toBeTruthy()

    await page.getByTestId('trade-desktop-ticket-toggle').click()
    await expect(ticketCol).toBeHidden()
    await page.waitForTimeout(50)
    const afterHideTicket = await chart.boundingBox()
    expect(afterHideTicket!.width).toBeGreaterThan(before!.width + 40)

    const afterTicket = afterHideTicket!.width
    await page.getByTestId('trade-desktop-book-toggle').click()
    await expect(page.getByTestId('trade-desktop-book-col')).toBeHidden()
    await page.waitForTimeout(50)
    const afterHideBook = await chart.boundingBox()
    expect(afterHideBook!.width).toBeGreaterThan(afterTicket + 20)

    await expect(page.getByTestId('trade-desktop-ticket-toggle')).toBeVisible()
    await expect(page.getByTestId('trade-desktop-book-toggle')).toBeVisible()
    await expect(page.getByTestId('trade-page-heading')).toBeVisible()
  })

  test('P12 tape is a bottom-row sibling of the chart; hide book expands chart (GitLab #561)', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')
    await expect(page.getByTestId('trade-desktop-workspace')).toBeVisible({ timeout: 90_000 })

    const chart = page.getByTestId('trade-desktop-chart-col')
    const tape = page.getByTestId('trade-desktop-tape-panel')
    const bookCol = page.getByTestId('trade-desktop-book-col')
    await expect(chart).toBeVisible()
    await expect(tape).toBeVisible()
    const chartBox = await chart.boundingBox()
    const tapeBox = await tape.boundingBox()
    expect(chartBox, 'chart box').toBeTruthy()
    expect(tapeBox, 'tape box').toBeTruthy()
    expect(tapeBox!.y, 'tape sits below the chart row').toBeGreaterThan(chartBox!.y + chartBox!.height - 8)
    expect(
      await chart.evaluate((el) => el.contains(document.querySelector('[data-testid="trade-desktop-tape-panel"]')))
    ).toBe(false)

    const before = chartBox!.width
    const ticketBefore = (await page.getByTestId('trade-desktop-ticket-col').boundingBox())!.width
    await page.getByTestId('trade-desktop-book-toggle').click()
    await expect(bookCol).toBeHidden()
    await page.waitForTimeout(50)
    const afterHideBook = await chart.boundingBox()
    expect(afterHideBook!.width).toBeGreaterThan(before + 40)
    const ticketAfter = await page.getByTestId('trade-desktop-ticket-col').boundingBox()
    expect(Math.abs(ticketAfter!.width - ticketBefore), 'ticket must not take the vacated book track').toBeLessThan(48)
    await expect(page.getByTestId('trade-desktop-book-toggle')).toBeVisible()
  })

  test('P13 1440 desktop has no resize handles; phone/tablet have none (GitLab #561)', async ({ page }) => {
    for (const size of [
      { width: 1440, height: 900 },
      { width: 820, height: 1180 },
      { width: 390, height: 844 },
    ] as const) {
      await page.setViewportSize(size)
      await page.goto('/trade')
      await page.waitForLoadState('networkidle')
      if (size.width >= 1024) {
        await expect(page.getByTestId('trade-desktop-workspace')).toBeVisible({ timeout: 90_000 })
      } else {
        await expect(page.getByTestId('trade-sub-lg-workspace')).toBeVisible({ timeout: 90_000 })
      }
      await expect(page.getByTestId('trade-ticket-resize-handle')).toHaveCount(0)
      await expect(page.getByTestId('trade-book-chart-resize-handle')).toHaveCount(0)
      await expect(page.getByTestId('trade-chart-tape-resize-handle')).toHaveCount(0)
      expect(await page.locator('[data-panel-resize-handle-id]').count()).toBe(0)
    }
  })

  test('P11 dark + light footer remains opaque', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    await page.goto('/trade')
    await page.waitForLoadState('networkidle')
    await expect(page.getByTestId('trade-desktop-workspace')).toBeVisible({ timeout: 90_000 })

    const footer = page.getByTestId('trade-ticket-submit-footer')
    for (const theme of ['dark', 'light'] as const) {
      await page.evaluate((t) => document.documentElement.setAttribute('data-theme', t), theme)
      await page.waitForTimeout(30)
      expect(await footer.evaluate(footerIsOpaque), `${theme} footer must be opaque`).toBe(true)
    }
  })
})
