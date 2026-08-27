import { type Locator, type Page } from '@playwright/test'
import { test, expect } from './fixtures/dev-wallet'
import { factoryAddressFromEnv } from './helpers/blacklist-lcd-mock'

type Box = { x: number; y: number; width: number; height: number }

function right(b: Box) {
  return b.x + b.width
}

function bottom(b: Box) {
  return b.y + b.height
}

/** GitLab #667 — Δ% chip sits with its headline, not under the next column. */
async function assertChipGroupedWithHeadline(page: Page, tile: Locator, chip: Locator, nextTile?: Locator) {
  await expect(tile).toBeVisible()
  await expect(chip).toBeVisible()
  const value = tile.locator('p.font-heading').first()
  await expect(value).toBeVisible()

  const tileId = await tile.getAttribute('data-testid')
  const chipId = await chip.getAttribute('data-testid')
  expect(tileId).toBeTruthy()
  expect(chipId).toBeTruthy()

  const geom = await page.evaluate(
    ({ tileSel, chipSel }) => {
      const tileEl = document.querySelector(tileSel)
      const chipEl = tileEl?.querySelector(chipSel)
      const valueEl = tileEl?.querySelector('p.font-heading')
      const clusterEl = tileEl?.querySelector('.stat-delta-cluster')
      if (!tileEl || !chipEl || !valueEl) return null
      const t = tileEl.getBoundingClientRect()
      const c = chipEl.getBoundingClientRect()
      const v = valueEl.getBoundingClientRect()
      const k = clusterEl?.getBoundingClientRect()
      return {
        tile: { x: t.left, y: t.top, width: t.width, height: t.height },
        chip: { x: c.left, y: c.top, width: c.width, height: c.height },
        value: { x: v.left, y: v.top, width: v.width, height: v.height },
        cluster: k ? { x: k.left, y: k.top, width: k.width, height: k.height } : null,
      }
    },
    { tileSel: `[data-testid="${tileId}"]`, chipSel: `[data-testid="${chipId}"]` }
  )
  expect(geom).toBeTruthy()
  const { tile: t, chip: c, value: v, cluster: k } = geom!

  expect(c.x).toBeGreaterThanOrEqual(v.x - 1)
  expect(c.x).toBeGreaterThanOrEqual(t.x - 1)
  expect(right(c)).toBeLessThanOrEqual(right(t) + 1)
  expect(c.y).toBeGreaterThanOrEqual(t.y - 1)
  expect(bottom(c)).toBeLessThanOrEqual(bottom(t) + 1)

  const clusterBox = k ?? c
  const sameLine = Math.abs(clusterBox.y - v.y) < 12
  if (sameLine) {
    expect(clusterBox.x - right(v)).toBeLessThan(28)
  } else {
    expect(clusterBox.y).toBeGreaterThanOrEqual(bottom(v) - 4)
    expect(clusterBox.x).toBeLessThanOrEqual(v.x + 8)
  }

  if (nextTile && (await nextTile.count())) {
    const nextBox = await nextTile.boundingBox()
    if (nextBox && Math.abs(nextBox.y - t.y) < 24) {
      expect(c.x).toBeLessThan(nextBox.x)
      expect(right(c)).toBeLessThanOrEqual(nextBox.x + 1)
    }
  }
}

async function assertProtocolDeltaGeometry(page: Page) {
  await expect(page.getByTestId('protocol-stat-liquidity')).toBeVisible({ timeout: 15_000 })
  const liq = page.getByTestId('protocol-stat-liquidity')
  await assertChipGroupedWithHeadline(
    page,
    liq,
    liq.getByTestId('protocol-stat-liquidity-24h'),
    page.getByTestId('protocol-stat-volume-24h')
  )
  await expect(liq.getByTestId('protocol-stat-liquidity-30d')).toHaveCount(0)
  await expect(liq).not.toContainText('30d')

  const vol24 = page.getByTestId('protocol-stat-volume-24h')
  const vol7d = page.getByTestId('protocol-stat-volume-7d')
  const vol30d = page.getByTestId('protocol-stat-volume-30d')
  await assertChipGroupedWithHeadline(page, vol24, vol24.getByTestId('protocol-stat-volume-24h-chg'), vol7d)
  await assertChipGroupedWithHeadline(page, vol7d, vol7d.getByTestId('protocol-stat-volume-7d-chg'), vol30d)
  await assertChipGroupedWithHeadline(page, vol30d, vol30d.getByTestId('protocol-stat-volume-30d-chg'))

  const feePanel = page.getByTestId('protocol-fee-stats')
  if (await feePanel.count()) {
    const f24 = page.getByTestId('protocol-stat-fees-24h')
    const f7 = page.getByTestId('protocol-stat-fees-7d')
    const f30 = page.getByTestId('protocol-stat-fees-30d')
    await assertChipGroupedWithHeadline(page, f24, f24.getByTestId('protocol-stat-fees-24h-chg'), f7)
    await assertChipGroupedWithHeadline(page, f7, f7.getByTestId('protocol-stat-fees-7d-chg'), f30)
    await assertChipGroupedWithHeadline(page, f30, f30.getByTestId('protocol-stat-fees-30d-chg'))
  }

  const tokens = page.getByTestId('protocol-stat-tokens')
  await expect(tokens).toBeVisible()
  const tokensText = await tokens.innerText()
  expect(tokensText).not.toMatch(/\d+\.00\b/)
  expect(tokensText).not.toMatch(/\d+\.000\b/)
}

/** GitLab #677 — x-axis labels every bar or every 2nd (hourly may widen). */
async function assertVolumeXAxisDensity(page: Page, grain: 'hourly' | 'daily' | 'monthly') {
  const chart = page.getByTestId('protocol-volume-daily-chart')
  if (!(await chart.count())) return
  const plot = page.getByTestId('protocol-volume-daily-bars')
  if (!(await plot.count())) return
  const barCount = await page.locator('[data-testid^="protocol-volume-bar-"]').count()
  if (barCount === 0) return
  const labelCount = await page.locator('[data-testid="protocol-volume-chart-xaxis"] text').count()
  expect(labelCount).toBeGreaterThan(0)
  expect(labelCount).toBeLessThanOrEqual(barCount)
  if (grain === 'hourly' && barCount > 12) {
    expect(labelCount).toBeGreaterThan(5)
  } else {
    expect(labelCount).toBeGreaterThanOrEqual(Math.ceil(barCount / 2))
  }
}

test.describe('Protocol page (GitLab #550 / #422)', () => {
  test('P1 stats card + oracle card + audit rows', async ({ page }) => {
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')

    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('protocol-global-stats')).toBeVisible()
    const feePanel = page.getByTestId('protocol-fee-stats')
    if (await feePanel.count()) {
      const statsBox = page.getByTestId('protocol-global-stats')
      const hubBox = page.getByTestId('protocol-dex-hub-prices')
      const statsPos = await statsBox.boundingBox()
      const feePos = await feePanel.boundingBox()
      const hubPos = await hubBox.boundingBox()
      expect(statsPos && feePos && statsPos.y < feePos.y).toBeTruthy()
      expect(feePos && hubPos && feePos.y < hubPos.y).toBeTruthy()
      await expect(page.getByTestId('protocol-stat-fees-24h')).toBeVisible()
      await expect(page.getByTestId('protocol-stat-fees-7d')).toBeVisible()
      await expect(page.getByTestId('protocol-stat-fees-30d')).toBeVisible()
      await expect(page.getByTestId('protocol-stat-fees-24h-chg')).toBeVisible()
    }
    await expect(page.getByTestId('protocol-stat-liquidity')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('protocol-stat-liquidity-24h')).toBeVisible()
    await expect(page.getByTestId('protocol-stat-liquidity-30d')).toHaveCount(0)
    await expect(page.getByTestId('protocol-stat-volume-24h')).toBeVisible()
    const dailyChart = page.getByTestId('protocol-volume-daily-chart')
    if (await dailyChart.count()) {
      await expect(dailyChart).toBeVisible()
      await expect(page.getByTestId('protocol-volume-grain-daily')).toBeVisible()
      await expect(page.getByTestId('protocol-volume-grain-hourly')).toBeVisible()
      await expect(page.getByTestId('protocol-volume-grain-monthly')).toBeVisible()
      await assertVolumeXAxisDensity(page, 'daily')
    }
    await expect(page.getByTestId('protocol-dex-hub-prices')).toBeVisible()
    await expect(page.getByTestId('protocol-dex-hub-custc')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('protocol-dex-hub-lunc')).toBeVisible()
    await expect(page.getByTestId('protocol-dex-hub-ust1')).toBeVisible()
    await expect(page.getByTestId('protocol-dex-hub-ustr')).toBeVisible()
    await expect(page.getByTestId('protocol-oracle')).toBeVisible()
    await expect(page.getByTestId('protocol-contract-addresses')).toBeVisible()

    let factory: string | null = null
    try {
      factory = factoryAddressFromEnv()
    } catch {
      factory = null
    }
    if (factory) {
      await expect(page.getByTestId('protocol-factory-address')).toContainText(factory)
      await expect(page.getByTestId('protocol-router-address')).toContainText(/^terra1/)
    } else {
      await expect(page.getByTestId('protocol-contract-addresses')).toContainText(/Factory|Not configured/i)
    }
  })

  test('P2 ticker chips update heading', async ({ page }) => {
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 30_000 })
    await expect(page.getByTestId('protocol-oracle-tab-ustc')).toBeVisible()
    await expect(page.getByTestId('protocol-oracle-tab-lunc')).toBeVisible()
    await expect(page.getByTestId('protocol-oracle-tab-vfdusd')).toBeVisible()
    await page.getByTestId('protocol-oracle-tab-lunc').click()
    await expect(page.getByRole('heading', { name: /LUNC \/ USD/i })).toBeVisible()
  })

  test('P3 one history table — no duplicate Recent USTC heading', async ({ page }) => {
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 30_000 })
    await expect(page.getByTestId('protocol-oracle')).toBeVisible()
    await expect(page.getByRole('heading', { name: /Recent USTC\/USD history/i })).toHaveCount(0)
    await expect(page.getByTestId('protocol-oracle')).toHaveCount(1)
  })

  test('P4 tablet 820×1180 cards stack; tabs usable', async ({ page }) => {
    await page.setViewportSize({ width: 820, height: 1180 })
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 30_000 })
    await expect(page.getByTestId('protocol-global-stats')).toBeVisible()
    await expect(page.getByTestId('protocol-oracle-tabs')).toBeVisible()
    await page.getByTestId('protocol-oracle-tab-vfdusd').click()
    await expect(page.getByRole('heading', { name: /^vFDUSD$/i })).toBeVisible()
    await expect(page.getByTestId('protocol-oracle-vfdusd-venus')).toBeVisible()
    await expect(page.getByRole('heading', { name: /1 vFDUSD Price/i })).toBeVisible()
    await expect(
      page.getByText('FDUSD reference price').or(page.getByText(/Failed to load oracle price/i))
    ).toBeVisible()
  })

  test('P5 phone 390×844 Protocol nav still works', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 30_000 })
    await expect(page.getByTestId('protocol-global-stats')).toBeVisible()
    await expect(page.getByTestId('protocol-stat-liquidity')).toBeVisible()
    await expect(page.getByTestId('protocol-stat-liquidity-24h')).toBeVisible()
    await expect(page.getByTestId('protocol-stat-liquidity-30d')).toHaveCount(0)
    const phoneFees = page.getByTestId('protocol-fee-stats')
    if (await phoneFees.count()) {
      await expect(phoneFees).toBeVisible()
      const feeBox = await phoneFees.boundingBox()
      const hubBox = await page.getByTestId('protocol-dex-hub-prices').boundingBox()
      expect(feeBox && hubBox && feeBox.y + feeBox.height <= hubBox.y + 8).toBeTruthy()
    }
    await expect(page.getByTestId('protocol-dex-hub-prices')).toBeVisible()
    await expect(page.getByTestId('protocol-dex-hub-lunc')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('protocol-oracle')).toBeVisible()
    await expect(page.getByTestId('protocol-contract-addresses')).toBeVisible()
  })
})

test.describe('Protocol Δ% grouping (GitLab #667)', () => {
  test('desktop 1280: chips sit with their own headline', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 900 })
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 30_000 })
    await assertProtocolDeltaGeometry(page)
  })

  test('tablet 820: 7d vol chip does not sit under 30d vol', async ({ page }) => {
    await page.setViewportSize({ width: 820, height: 1180 })
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 30_000 })
    await assertProtocolDeltaGeometry(page)
  })

  test('phone 390: wrap stays inside the tile', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 30_000 })
    await assertProtocolDeltaGeometry(page)
  })
})

test.describe('Protocol leftovers (GitLab #677)', () => {
  test('Hourly / Daily / Monthly x-axis meets density floor', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 900 })
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 30_000 })
    const chart = page.getByTestId('protocol-volume-daily-chart')
    if (!(await chart.count())) return
    await expect(chart).toBeVisible()
    await assertVolumeXAxisDensity(page, 'daily')
    await page.getByTestId('protocol-volume-grain-hourly').click()
    await expect(chart).toContainText(/UTC hour/)
    await assertVolumeXAxisDensity(page, 'hourly')
    await page.getByTestId('protocol-volume-grain-monthly').click()
    await expect(chart).toContainText(/UTC calendar month/)
    await assertVolumeXAxisDensity(page, 'monthly')
  })
})

test.describe('Protocol UTC metric toggle (GitLab #689)', () => {
  test('Volume / Liquidity / Fees tabs stay in Global stats', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 900 })
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 30_000 })
    const stats = page.getByTestId('protocol-global-stats')
    await expect(stats).toBeVisible()
    const chart = page.getByTestId('protocol-volume-daily-chart')
    if (!(await chart.count())) return
    await expect(chart).toBeVisible()
    await expect(page.getByTestId('protocol-utc-metric-volume')).toBeVisible()
    await expect(page.getByTestId('protocol-utc-metric-liquidity')).toBeVisible()
    await expect(page.getByTestId('protocol-utc-metric-fees')).toBeVisible()
    await expect(page.getByTestId('protocol-volume-grain-hourly')).toBeVisible()
    await expect(page.getByTestId('protocol-volume-grain-daily')).toBeVisible()
    await expect(page.getByTestId('protocol-volume-grain-monthly')).toBeVisible()
    await page.getByTestId('protocol-utc-metric-liquidity').click()
    await expect(chart).toContainText(/UTC calendar/)
    const yAxis = page.getByTestId('protocol-volume-chart-yaxis')
    if (await yAxis.count()) {
      await expect(yAxis).toContainText('$')
      const bar = page.getByTestId('protocol-volume-bar-0')
      if (await bar.count()) {
        await bar.hover()
        await expect(page.getByTestId('protocol-volume-chart-tooltip')).toBeVisible()
      }
    }
    await page.getByTestId('protocol-utc-metric-fees').click()
    await expect(chart).toContainText(/UTC calendar/)
    await expect(page.getByTestId('protocol-stat-liquidity-30d')).toHaveCount(0)
    const fees = page.getByTestId('protocol-fee-stats')
    const feeCount = await fees.count()
    if (feeCount) {
      await expect(fees).toBeVisible()
      await expect(page.getByTestId('protocol-stat-fees-24h')).toBeVisible()
      await expect(page.getByTestId('protocol-stat-fees-7d')).toBeVisible()
      await expect(page.getByTestId('protocol-stat-fees-30d')).toBeVisible()
    }
    const chartBox = await chart.boundingBox()
    const feesBox = feeCount ? await fees.boundingBox() : null
    if (chartBox && feesBox) {
      expect(chartBox.y + chartBox.height).toBeLessThanOrEqual(feesBox.y + 8)
    }
  })

  test('phone 390: metric tabs do not nest card-glass', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/protocol')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.getByRole('heading', { name: /^protocol$/i })).toBeVisible({ timeout: 30_000 })
    await expect(page.getByTestId('protocol-stat-liquidity-24h')).toBeVisible()
    await expect(page.getByTestId('protocol-stat-liquidity-30d')).toHaveCount(0)
    const chart = page.getByTestId('protocol-volume-daily-chart')
    if (!(await chart.count())) return
    await expect(chart).toBeVisible()
    const hasGlass = await chart.evaluate(
      (el) => el.classList.contains('card-glass') || Boolean(el.querySelector('.card-glass'))
    )
    expect(hasGlass).toBeFalsy()
    await page.getByTestId('protocol-utc-metric-liquidity').click()
    await expect(chart).toBeVisible()
  })
})
