import { test, expect, type Locator, type Page } from '@playwright/test'

import { skipIfLcdUnreachable } from './helpers/chain'
import { requireDualCwPair } from './helpers/hybrid-e2e'
import { gotoAndCaptureFactoryPairsPage } from './helpers/lcd'
import { swapActionPanel } from './helpers/swap-ui'

/**
 * GitLab #528 — Slippage protection chips stay one aligned group.
 * Needs LocalTerra + a factory pair so `/trade` Market mounts the ticket chips.
 * Uses the factory-pairs capture helper (same as other trade E2E) so host :1317
 * hang / LCD-proxy setups still resolve a pair.
 */

const TRADE = {
  group: 'trade-market-slippage-presets',
  label: 'trade-market-slippage-presets-label',
  p05: 'trade-market-slippage-preset-0.5',
  p1: 'trade-market-slippage-preset-1',
  p5: 'trade-market-slippage-preset-5',
} as const

const SWAP = {
  group: 'swap-slippage-presets',
  label: 'swap-slippage-presets-label',
  p05: 'swap-slippage-preset-0.5',
  p1: 'swap-slippage-preset-1',
  p5: 'swap-slippage-preset-5',
  custom: 'swap-slippage-custom',
} as const

type ChipIds = typeof TRADE | typeof SWAP

async function dismissOnboarding(page: Page) {
  const dismiss = page.getByRole('button', { name: /Dismiss getting started/i })
  if (await dismiss.isVisible().catch(() => false)) {
    await dismiss.click()
  }
}

async function openTradeMarket(page: Page, request: Parameters<typeof skipIfLcdUnreachable>[0]) {
  await skipIfLcdUnreachable(request)
  const pairs = await gotoAndCaptureFactoryPairsPage(page, '/trade')
  const { pair } = requireDualCwPair(pairs)
  await page.goto(`/trade/${pair.contract_addr}`)
  await dismissOnboarding(page)
  const marketTab = page.getByTestId('trade-order-tab-market')
  await expect(marketTab).toBeVisible({ timeout: 90_000 })
  await marketTab.click()
  await page.getByTestId('trade-market-advanced-toggle').click()
  await expect(page.getByTestId(TRADE.p05)).toBeVisible({ timeout: 90_000 })
}

async function openSwapSettings(page: Page, request: Parameters<typeof skipIfLcdUnreachable>[0]) {
  await skipIfLcdUnreachable(request)
  await gotoAndCaptureFactoryPairsPage(page, '/')
  const panel = swapActionPanel(page)
  await expect(panel.getByRole('button', { name: 'Settings' })).toBeVisible({ timeout: 30_000 })
  await panel.getByRole('button', { name: 'Settings' }).click()
  await expect(page.locator('#swap-slippage-settings')).toBeVisible({ timeout: 30_000 })
  await expect(page.getByTestId(SWAP.p05)).toBeVisible()
}

async function box(locator: Locator) {
  const b = await locator.boundingBox()
  expect(b, `bounding box for ${await locator.getAttribute('data-testid')}`).toBeTruthy()
  return b!
}

async function assertAlignedGroup(page: Page, ids: ChipIds) {
  const c05 = page.getByTestId(ids.p05)
  const c1 = page.getByTestId(ids.p1)
  const c5 = page.getByTestId(ids.p5)
  const label = page.getByTestId(ids.label)
  await expect(c05).toBeVisible()
  await expect(c1).toBeVisible()
  await expect(c5).toBeVisible()
  await expect(label).toBeVisible()

  const b05 = await box(c05)
  const b1 = await box(c1)
  const b5 = await box(c5)
  const bl = await box(label)

  expect(Math.abs(b05.y - b1.y), '0.5% and 1% share y').toBeLessThanOrEqual(2)
  expect(Math.abs(b1.y - b5.y), '1% and 5% share y').toBeLessThanOrEqual(2)
  expect(Math.abs(b05.y + b05.height - (b1.y + b1.height)), '0.5% and 1% share bottom').toBeLessThanOrEqual(2)
  expect(bl.y + bl.height, 'label sits above the chip row').toBeLessThanOrEqual(b05.y + 4)

  const gap01 = b1.x - (b05.x + b05.width)
  const gap15 = b5.x - (b1.x + b1.width)
  expect(gap01, '0.5% and 1% must not overlap').toBeGreaterThanOrEqual(0)
  expect(gap15, '1% and 5% must not overlap').toBeGreaterThanOrEqual(0)
}

async function assertElementFromPointHitsChip(page: Page, testId: string) {
  const chip = page.getByTestId(testId)
  await chip.evaluate((el) => el.scrollIntoView({ block: 'center', inline: 'nearest' }))
  await page.waitForTimeout(50)
  const hit = await chip.evaluate((el) => {
    const r = el.getBoundingClientRect()
    const x = Math.min(Math.max(r.left + r.width / 2, 0), window.innerWidth - 1)
    const y = Math.min(Math.max(r.top + r.height / 2, 0), window.innerHeight - 1)
    const top = document.elementFromPoint(x, y)
    return Boolean(top?.closest(`[data-testid="${el.getAttribute('data-testid')}"]`))
  })
  expect(hit, `elementFromPoint at ${testId} center must hit that chip`).toBe(true)
}

test.describe('Slippage protection preset alignment (GitLab #528)', () => {
  test('P1 phone 390×844 Market chips share a baseline', async ({ page, request }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await openTradeMarket(page, request)
    await assertAlignedGroup(page, TRADE)
  })

  test('P2 tablet 820×1180 Market chips share a baseline', async ({ page, request }) => {
    await page.setViewportSize({ width: 820, height: 1180 })
    await openTradeMarket(page, request)
    await assertAlignedGroup(page, TRADE)
  })

  test('P3 desktop 1280×720 Market chips share a baseline', async ({ page, request }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    await openTradeMarket(page, request)
    await assertAlignedGroup(page, TRADE)
  })

  test('P4 narrow desktop ticket still keeps a 3-up group', async ({ page, request }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    await openTradeMarket(page, request)
    const workspace = page.getByTestId('trade-desktop-workspace')
    await expect(workspace).toBeVisible({ timeout: 90_000 })
    const handles = workspace.locator('[data-panel-resize-handle-id]')
    const handleCount = await handles.count()
    if (handleCount > 0) {
      const handle = handles.last()
      const hb = await handle.boundingBox()
      if (hb) {
        await page.mouse.move(hb.x + hb.width / 2, hb.y + hb.height / 2)
        await page.mouse.down()
        await page.mouse.move(hb.x + 420, hb.y + hb.height / 2, { steps: 8 })
        await page.mouse.up()
      }
    }
    await assertAlignedGroup(page, TRADE)
    const b05 = await box(page.getByTestId(TRADE.p05))
    const b1 = await box(page.getByTestId(TRADE.p1))
    expect(b05.y, '0.5% must not sit on a shorter row than 1%').toBeLessThanOrEqual(b1.y + 2)
  })

  test('P5 click 0.5 → 1 → 5; chips do not overlap', async ({ page, request }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await openTradeMarket(page, request)
    await page.getByTestId(TRADE.p05).click()
    await expect(page.getByTestId(TRADE.p05)).toHaveClass(/tab-glass-active/)
    await page.getByTestId(TRADE.p1).click()
    await expect(page.getByTestId(TRADE.p1)).toHaveClass(/tab-glass-active/)
    await expect(page.getByTestId(TRADE.p05)).toHaveClass(/tab-glass-inactive/)
    await page.getByTestId(TRADE.p5).click()
    await expect(page.getByTestId(TRADE.p5)).toHaveClass(/tab-glass-active/)
    await assertAlignedGroup(page, TRADE)
  })

  test('P6 elementFromPoint at each chip center hits that chip', async ({ page, request }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await openTradeMarket(page, request)
    await page.getByTestId(TRADE.p05).scrollIntoViewIfNeeded()
    await assertElementFromPointHitsChip(page, TRADE.p05)
    await assertElementFromPointHitsChip(page, TRADE.p1)
    await assertElementFromPointHitsChip(page, TRADE.p5)
  })

  test('P7 Swap Settings 390×844: presets share y; Custom is not between chips', async ({ page, request }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await openSwapSettings(page, request)
    await assertAlignedGroup(page, SWAP)
    const group = page.getByTestId(SWAP.group)
    const custom = page.getByTestId(SWAP.custom)
    const insideGroup = await custom.evaluate(
      (el, groupTestId) => Boolean(el.closest(`[data-testid="${groupTestId}"]`)),
      SWAP.group
    )
    expect(insideGroup, 'Custom must sit outside the chip group').toBe(false)
    const cb = await box(custom)
    const gb = await box(group)
    expect(cb.y, 'Custom is at or below the chip group').toBeGreaterThanOrEqual(gb.y - 2)
    const b05 = await box(page.getByTestId(SWAP.p05))
    const b1 = await box(page.getByTestId(SWAP.p1))
    const betweenChips = cb.x < b1.x && cb.x + cb.width > b05.x + b05.width && Math.abs(cb.y - b05.y) <= 4
    expect(betweenChips, 'Custom must not sit between 0.5% and 1%').toBe(false)
  })

  test('P8 light + dark keep phone alignment', async ({ page, request }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await page.addInitScript(() => {
      localStorage.setItem('cl8y-dex-theme', 'dark')
    })
    await openTradeMarket(page, request)
    await assertAlignedGroup(page, TRADE)
    await expect(page.getByTestId(TRADE.p5)).toHaveClass(/tab-glass-active/)

    const light = page.getByRole('button', { name: 'Light theme' }).first()
    if (await light.isVisible().catch(() => false)) {
      await light.click()
    } else {
      await page.evaluate(() => {
        document.documentElement.setAttribute('data-theme', 'light')
        localStorage.setItem('cl8y-dex-theme', 'light')
      })
    }
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')
    await assertAlignedGroup(page, TRADE)
  })

  test('P9 disconnected chips stay aligned and clickable', async ({ page, request }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await openTradeMarket(page, request)
    await assertAlignedGroup(page, TRADE)
    await page.getByTestId(TRADE.p05).click()
    await expect(page.getByTestId(TRADE.p05)).toHaveClass(/tab-glass-active/)
    const cta = page.getByTestId('trade-market-submit')
    await expect(cta).toBeVisible()
  })

  test('P10 Market ↔ Limit preserves selection and alignment', async ({ page, request }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await openTradeMarket(page, request)
    await page.getByTestId(TRADE.p1).click()
    await expect(page.getByTestId(TRADE.p1)).toHaveClass(/tab-glass-active/)
    await page.getByTestId('trade-order-tab-limit').click()
    await expect(page.getByTestId('trade-order-tab-limit')).toHaveAttribute('aria-selected', 'true')
    await page.getByTestId('trade-order-tab-market').click()
    await page.getByTestId('trade-market-advanced-toggle').click()
    await expect(page.getByTestId(TRADE.p1)).toHaveClass(/tab-glass-active/)
    await assertAlignedGroup(page, TRADE)
  })
})
