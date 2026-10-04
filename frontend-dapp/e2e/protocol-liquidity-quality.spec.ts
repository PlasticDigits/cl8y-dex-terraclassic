import { expect, test } from '@playwright/test'
import type { IndexerOverview } from '../src/types'

const observation = new Date().toISOString()
const overview: IndexerOverview = {
  total_volume_24h: '0',
  total_volume_24h_usd: '1234',
  total_trades_24h: 0,
  pair_count: 22,
  token_count: 12,
  total_liquidity_usd: '218',
  liquidity_quality: 'partial',
  liquidity_as_of: observation,
  liquidity_max_age_seconds: 300,
  liquidity_change_24h_pct: '-100',
}

for (const width of [390, 1280]) {
  test(`liquidity gaps and lower-bound headline, tap and keyboard at ${width}px (#1380)`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 })
    await page.route('**/api/v1/**', async (route) => {
      const url = new URL(route.request().url())
      let body: unknown = null
      if (url.pathname.endsWith('/overview')) body = { ...overview, liquidity_as_of: new Date().toISOString() }
      else if (url.pathname.includes('/protocol/volume/daily'))
        body = {
          grain: 'daily',
          limit: 7,
          timezone: 'UTC',
          methodology: 'protocol_catalog',
          series: [{ utc_day: '2026-10-04', volume_usd: '1234', trade_count: 1 }],
        }
      else if (url.pathname.includes('/protocol/liquidity/daily'))
        body = {
          grain: url.searchParams.get('grain'),
          limit: 7,
          timezone: 'UTC',
          methodology: 'protocol_catalog',
          series: [
            { utc_day: '2026-10-01', liquidity_usd: '250000', quality: 'fresh' },
            { utc_day: '2026-10-02', liquidity_usd: '218', quality: 'partial', as_of: observation },
            { utc_day: '2026-10-03', liquidity_usd: '0', quality: 'unavailable' },
            { utc_day: '2026-10-04', liquidity_usd: '0', quality: 'verified_empty' },
            { utc_day: '2026-10-05', liquidity_usd: '218.57', priced_pair_count: 2 },
          ],
        }
      if (body) await route.fulfill({ json: body })
      else await route.fulfill({ status: 404, json: { error: 'Unavailable fixture' } })
    })
    await page.goto('/protocol')
    const liquidity = page.getByTestId('protocol-stat-liquidity')
    await expect(liquidity).toBeVisible({ timeout: 60_000 })
    await expect(liquidity).toContainText('≥ $')
    await expect(liquidity).toContainText('Partial · lower bound')
    await expect(liquidity).not.toContainText('-100%')
    await expect(page.getByTestId('protocol-stat-volume-24h')).toContainText('$')
    await page.getByTestId('protocol-utc-metric-liquidity').click()
    const gap = page.getByTestId('protocol-volume-bar-1')
    await expect(gap).toHaveAttribute('stroke-dasharray', '3 3')
    await gap.click()
    const tooltip = page.getByTestId('protocol-volume-chart-tooltip')
    await expect(tooltip).toContainText('Partial · lower bound: ≥ $')
    await expect(tooltip).toContainText(`As of ${observation}`)
    await gap.focus()
    await page.keyboard.press('Escape')
    await expect(tooltip).toHaveCount(0)
    await page.getByTestId('protocol-volume-bar-2').focus()
    await expect(tooltip).toContainText('Unavailable · liquidity unknown')
    await expect(page.getByTestId('protocol-volume-bar-3')).toHaveAttribute('aria-label', /\$0.*Verified empty/)
    const legacyPositive = page.getByTestId('protocol-volume-bar-4')
    await expect(legacyPositive).toHaveAttribute('stroke-dasharray', '3 3')
    await expect(legacyPositive).toHaveAttribute('aria-label', /Unknown liquidity/)
    await legacyPositive.focus()
    await expect(tooltip).toContainText('Unknown liquidity')
    await expect(tooltip).not.toContainText('$218')
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1)
    expect(overflow).toBe(false)
    await page.screenshot({ path: `/tmp/issue-1380-liquidity-${width}.png`, fullPage: true })
  })
}

test('cached overview ages independently; recovery and genuine zero remain visible (#1380)', async ({ page }) => {
  let phase = 0
  const frozen = new Date('2026-10-04T08:00:00Z')
  await page.clock.setFixedTime(frozen)
  await page.route('**/api/v1/**', async (route) => {
    if (new URL(route.request().url()).pathname.endsWith('/overview')) {
      await route.fulfill({
        json: {
          ...overview,
          liquidity_quality: phase === 2 ? 'verified_empty' : 'fresh',
          total_liquidity_usd: phase === 2 ? '0' : '250000',
          liquidity_as_of: frozen.toISOString(),
          liquidity_max_age_seconds: phase === 0 ? 2 : 300,
          liquidity_change_24h_pct: phase === 2 ? null : '1',
        },
      })
    } else await route.fulfill({ status: 404, json: {} })
  })
  await page.goto('/protocol')
  const liquidity = page.getByTestId('protocol-stat-liquidity')
  await expect(liquidity).toBeVisible({ timeout: 60_000 })
  await expect(liquidity).toContainText('$250K')
  await page.clock.setFixedTime(new Date(frozen.getTime() + 3000))
  await expect(liquidity).toContainText('Stale · current liquidity unknown')
  await expect(liquidity).not.toContainText('$250K')
  phase = 1
  await page.reload()
  await expect(page.getByTestId('protocol-global-stats')).toBeVisible({ timeout: 30_000 })
  await expect(liquidity).toContainText('$250K')
  phase = 2
  await page.reload()
  await expect(page.getByTestId('protocol-global-stats')).toBeVisible({ timeout: 30_000 })
  await expect(liquidity).toContainText('$0')
  await expect(liquidity).toContainText('Verified empty')
})
