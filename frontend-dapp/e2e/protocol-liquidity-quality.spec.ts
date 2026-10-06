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
            {
              utc_day: '2026-10-06',
              liquidity_usd: null,
              reported_liquidity_usd: '249000',
              quality: 'unknown',
              reasons: { legacy_unqualified: 1 },
            },
          ],
        }
      if (body) await route.fulfill({ json: body })
      else await route.fulfill({ status: 404, json: { error: 'Unavailable fixture' } })
    })
    await page.goto('/protocol')
    const liquidity = page.getByTestId('protocol-stat-liquidity')
    await expect(liquidity).toBeVisible({ timeout: 60_000 })
    await expect(liquidity).toContainText('≥ $')
    await expect(liquidity).not.toContainText(/Fresh|As of|Partial ·/)
    const info = page.getByRole('button', { name: 'Liquidity details' })
    await info.click()
    await expect(page.getByRole('tooltip')).toContainText('This is a lower bound.')
    await expect(page.getByRole('tooltip')).toContainText('Updated')
    await expect(page.getByRole('tooltip')).not.toContainText('2026-')
    await page.keyboard.press('Escape')
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
    const estimate = page.getByTestId('protocol-volume-bar-5')
    await expect(estimate).toHaveAttribute('data-historical-estimate', 'true')
    await expect(estimate).toHaveAttribute('data-quality', 'unknown')
    await expect(estimate).toHaveAttribute('stroke-dasharray', '2 2')
    await expect(estimate).toHaveAttribute('aria-label', /≈.*Historical estimate/)
    expect(Number(await estimate.getAttribute('height'))).toBeGreaterThan(0)
    await estimate.focus()
    await expect(tooltip).toContainText('Historical estimate')
    await page.keyboard.press('Escape')
    await page.getByRole('button', { name: 'Chart details' }).click()
    await expect(page.getByRole('tooltip')).toContainText('Hatched bars are historical estimates.')
    await page.keyboard.press('Escape')
    const visible = await page.locator('body').innerText()
    expect(visible).not.toMatch(
      /USD volume and pool TVL use the same|UTC calendar day|Dashed gaps mean|not swap settlement, TWAP/
    )
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1)
    expect(overflow).toBe(false)
    await page.screenshot({ path: `/tmp/issue-1380-liquidity-${width}.png`, fullPage: true })
  })
}

test('last verified total survives expiry, outage and reload; decreases and genuine zero replace it', async ({
  page,
}) => {
  let phase = 0
  const frozen = new Date('2026-10-04T08:00:00Z')
  await page.clock.setFixedTime(frozen)
  await page.route('**/api/v1/**', async (route) => {
    if (new URL(route.request().url()).pathname.endsWith('/overview')) {
      if (phase === 4) {
        await route.abort('failed')
        return
      }
      await route.fulfill({
        json: {
          ...overview,
          liquidity_quality:
            phase === 2 ? 'verified_empty' : phase === 3 ? 'unavailable' : phase === 5 ? 'partial' : 'fresh',
          total_liquidity_usd:
            phase === 2 ? '0' : phase === 3 ? null : phase === 5 ? '999' : phase === 1 ? '120000' : '250000',
          liquidity_as_of:
            phase === 0 || phase === 3 || phase === 5
              ? frozen.toISOString()
              : new Date(frozen.getTime() + 3000).toISOString(),
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
  await expect(page.getByTestId('protocol-stat-liquidity-24h')).toContainText('%')
  const saved = await page.evaluate(() =>
    Object.entries(localStorage).filter(([key]) => key.startsWith('cl8y:protocol-liquidity:'))
  )
  expect(saved).toHaveLength(1)
  expect(Object.keys(JSON.parse(saved[0][1])).sort()).toEqual(['asOf', 'quality', 'value'])
  await page.clock.setFixedTime(new Date(frozen.getTime() + 3000))
  await expect(page.getByTestId('protocol-stat-liquidity-24h')).toHaveCount(0)
  await expect(liquidity).toContainText('$250K')
  await page.getByRole('button', { name: 'Liquidity details' }).click()
  await expect(page.getByRole('tooltip')).toContainText('Last verified total. Updated 3 seconds ago.')
  await expect(page.getByRole('tooltip')).not.toContainText(/expired|too old|stale/i)
  await page.keyboard.press('Escape')
  for (const degraded of [3, 4, 5]) {
    phase = degraded
    await page.reload()
    await expect(liquidity).toContainText('$250K')
    await expect(liquidity).not.toContainText(/expired|stale|≥/i)
    await expect(page.getByTestId('protocol-stat-liquidity-24h')).toHaveCount(0)
  }
  phase = 1
  await page.reload()
  await expect(liquidity).toContainText('$120K')
  phase = 2
  await page.reload()
  await expect(liquidity).toContainText('$0')
  await page.getByRole('button', { name: 'Liquidity details' }).click()
  await expect(page.getByRole('tooltip')).toContainText('No liquidity was found in the checked pools.')
  await page.keyboard.press('Escape')
  phase = 3
  await page.reload()
  await expect(liquidity).toContainText('$0')
  await page.getByRole('button', { name: 'Liquidity details' }).click()
  await expect(page.getByRole('tooltip')).toContainText('Last verified total.')
  await page.screenshot({ path: '/tmp/dex-ops-20261006/liquidity/last-known-zero-reload.png', fullPage: true })
})
