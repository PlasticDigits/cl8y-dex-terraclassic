import { expect, test } from '@playwright/test'
import { homedir } from 'node:os'
import { join } from 'node:path'
import type { IndexerOverview } from '../src/types'

const evidenceDir = join(homedir(), '.local/state/cl8y-dex/release-20261007/qa')
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
  test(`plain liquidity bars and retained headline, tap and keyboard at ${width}px (#1380)`, async ({ page }) => {
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
            { utc_day: '2026-10-03', liquidity_usd: null, quality: 'unavailable' },
            { utc_day: '2026-10-04', liquidity_usd: '0', quality: 'verified_empty' },
            { utc_day: '2026-10-05', liquidity_usd: '218.57', priced_pair_count: 2 },
            {
              utc_day: '2026-10-06',
              liquidity_usd: null,
              reported_liquidity_usd: '249000',
              quality: 'unknown',
              reasons: { legacy_unqualified: 1 },
            },
            // Dense phone series verifies label spacing while every available bar remains.
            ...Array.from({ length: 16 }, (_, index) => ({
              utc_day: `2026-10-${String(index + 7).padStart(2, '0')}`,
              liquidity_usd: String(225000 + index * 1000),
              quality: 'fresh',
            })),
          ],
        }
      if (body) await route.fulfill({ json: body })
      else await route.fulfill({ status: 404, json: { error: 'Unavailable fixture' } })
    })
    await page.goto('/protocol')
    const liquidity = page.getByTestId('protocol-stat-liquidity')
    await expect(liquidity).toBeVisible({ timeout: 60_000 })
    await expect(liquidity).toContainText('$218')
    await expect(liquidity).not.toContainText(/Fresh|As of|Partial ·/)
    await expect(page.getByRole('button', { name: 'Liquidity details' })).toHaveCount(0)
    await expect(liquidity).not.toContainText(/≥|Updated|As of|partial|fresh/i)
    await expect(liquidity).not.toContainText('-100%')
    await expect(page.getByTestId('protocol-stat-volume-24h')).toContainText('$')
    await page.getByTestId('protocol-utc-metric-liquidity').click()
    const partial = page.getByTestId('protocol-volume-bar-1')
    await expect(partial).toHaveAttribute('fill', 'var(--accent)')
    await partial.click()
    const tooltip = page.getByTestId('protocol-volume-chart-tooltip')
    await expect(tooltip).toHaveText(/^Oct 2, 2026 \$/)
    await expect(tooltip).not.toContainText(/quality|partial|snapshot|As of|UTC|fresh|≈|≥/i)
    await partial.focus()
    await page.keyboard.press('Escape')
    await expect(tooltip).toHaveCount(0)
    await expect(page.getByTestId('protocol-volume-bar-2')).toHaveAttribute('aria-label', 'Oct 4, 2026 $0')
    const reported = page.getByTestId('protocol-volume-bar-4')
    await expect(reported).toHaveAttribute('aria-label', 'Oct 6, 2026 $249K')
    await reported.focus()
    await expect(tooltip).toHaveText('Oct 6, 2026 $249K')
    await page.keyboard.press('Escape')
    const bars = page.locator('[data-testid^="protocol-volume-bar-"]')
    await expect(bars).toHaveCount(21)
    for (const bar of await bars.all()) {
      await expect(bar).toHaveAttribute('fill', 'var(--accent)')
      await expect(bar).not.toHaveAttribute('stroke-dasharray')
      await expect(bar).not.toHaveAttribute('data-quality')
    }
    await expect(page.getByRole('button', { name: 'Chart details' })).toHaveCount(0)
    const labels = await page
      .getByTestId('protocol-volume-chart-xaxis')
      .locator('text')
      .evaluateAll((nodes) =>
        nodes.map((node) => ({ left: node.getBoundingClientRect().left, right: node.getBoundingClientRect().right }))
      )
    for (let i = 1; i < labels.length; i++) expect(labels[i].left - labels[i - 1].right).toBeGreaterThanOrEqual(6)

    const visible = await page.locator('body').innerText()
    expect(visible).not.toMatch(
      /USD volume and pool TVL use the same|UTC calendar day|Dashed gaps mean|not swap settlement, TWAP/
    )
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1)
    expect(overflow).toBe(false)
    await page.screenshot({ path: join(evidenceDir, `issue-1380-liquidity-${width}.png`), fullPage: true })
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
  phase = 3
  await page.reload()
  await expect(liquidity).toContainText('$0')
  await page.screenshot({ path: join(evidenceDir, 'last-known-zero-reload.png'), fullPage: true })
})

test('first partial amount survives TTL, null, failure and reload until a complete total arrives', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 900 })
  const frozen = new Date('2026-10-04T08:00:00Z')
  await page.clock.setFixedTime(frozen)
  let phase: 'initial' | 'null' | 'failed' | 'lower' | 'complete' | 'partial' = 'initial'
  await page.route('**/api/v1/**', async (route) => {
    if (!new URL(route.request().url()).pathname.endsWith('/overview')) {
      await route.fulfill({ status: 404, json: {} })
      return
    }
    if (phase === 'failed') {
      await route.abort('failed')
      return
    }
    await route.fulfill({
      json: {
        ...overview,
        liquidity_quality: phase === 'complete' ? 'fresh' : phase === 'null' ? 'unavailable' : 'partial',
        total_liquidity_usd:
          phase === 'null' ? null : phase === 'lower' ? '120000' : phase === 'complete' ? '110000' : '206000',
        liquidity_as_of: phase === 'initial' ? frozen.toISOString() : new Date(frozen.getTime() + 3000).toISOString(),
        liquidity_max_age_seconds: phase === 'initial' ? 2 : 300,
      },
    })
  })
  await page.goto('/protocol')
  const liquidity = page.getByTestId('protocol-stat-liquidity')
  await expect(liquidity).toContainText('$206K', { timeout: 60_000 })
  await expect(page.getByTestId('protocol-stat-liquidity-24h')).toHaveCount(0)
  await expect
    .poll(() =>
      page.evaluate(() =>
        Object.entries(localStorage).some(
          ([key, value]) => key.startsWith('cl8y:protocol-liquidity:') && JSON.parse(value).quality === 'partial'
        )
      )
    )
    .toBe(true)
  await page.clock.setFixedTime(new Date(frozen.getTime() + 3000))
  await expect(liquidity).toContainText('$206K')
  await expect(liquidity).not.toContainText(/≥|Updated|As of|partial|fresh/i)
  await expect(page.getByRole('button', { name: 'Liquidity details' })).toHaveCount(0)
  for (const next of ['null', 'failed'] as const) {
    phase = next
    await page.reload()
    await expect(liquidity).toContainText('$206K')
    await expect(liquidity).not.toContainText(/≥|Updated|As of|partial|fresh/i)
    await expect(liquidity).not.toContainText(/expired|stale/i)
    await expect(liquidity.locator('.animate-pulse')).toHaveCount(0)
    await expect(page.getByTestId('protocol-stat-liquidity-24h')).toHaveCount(0)
  }
  phase = 'lower'
  await page.reload()
  await expect(liquidity).toContainText('$120K')
  phase = 'complete'
  await page.reload()
  await expect(liquidity).toContainText('$110K')
  await expect(liquidity).not.toContainText('≥')
  await expect(page.getByTestId('protocol-stat-liquidity-24h')).toContainText('%')
  phase = 'partial'
  await page.reload()
  await expect(liquidity).toContainText('$110K')
  await expect(liquidity).not.toContainText('≥')
  await expect(page.getByTestId('protocol-stat-liquidity-24h')).toHaveCount(0)
  await page.screenshot({ path: join(evidenceDir, 'partial-to-complete-retention-phone.png'), fullPage: true })
})
