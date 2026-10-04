import { test, expect, type Page } from '@playwright/test'

const STORAGE_KEY = 'cl8y-dex-number-display-mode'

/** LCD reads only: mount real Settings without chain data or synthetic DOM. */
async function mockSettingsReads(page: Page) {
  await page.route('**/*', async (route) => {
    const url = new URL(route.request().url())
    const smart = url.pathname.match(/\/contract\/[^/]+\/smart\/([^/]+)/)
    if (smart) {
      const query = JSON.parse(Buffer.from(decodeURIComponent(smart[1]), 'base64').toString('utf8'))
      const data = query.pairs
        ? { pairs: [] }
        : query.token_info
          ? { name: 'Fixture', symbol: 'FIX', decimals: 6, total_supply: '0' }
          : query.config
            ? { paused: false, fee_wrap_bps: 0, fee_unwrap_bps: 0 }
            : query.balance
              ? { balance: '0' }
              : {}
      return route.fulfill({ contentType: 'application/json', body: JSON.stringify({ data }) })
    }
    if (url.pathname.startsWith('/api/') || url.pathname.includes('/route/solve')) {
      return route.fulfill({ status: 503, contentType: 'application/json', body: '{"error":"mock indexer offline"}' })
    }
    return route.continue()
  })
}

async function openSettings(page: Page) {
  await expect(page.getByRole('button', { name: 'Settings', exact: true })).toBeVisible({ timeout: 90_000 })
  await page.getByRole('button', { name: 'Settings', exact: true }).click()
  await expect(page.locator('#swap-slippage-settings')).toBeVisible()
}

test.describe('Number display preference (#1259, display-only)', () => {
  test.beforeEach(async ({ page }) => {
    await mockSettingsReads(page)
  })

  for (const theme of ['dark', 'light'] as const) {
    for (const width of [375, 1280]) {
      test(`${theme} ${width}px: Compact default, Full persistence, plain input and URL`, async ({
        page,
      }, testInfo) => {
        await page.setViewportSize({ width, height: 850 })
        await page.addInitScript((value) => localStorage.setItem('cl8y-dex-theme', value), theme)
        await page.goto('/')
        await openSettings(page)
        const compact = page.getByTestId('swap-number-display-compact')
        const full = page.getByTestId('swap-number-display-full')
        const preview = page.getByTestId('swap-number-display-preview')
        await expect(compact).toHaveAttribute('aria-pressed', 'true')
        await expect(preview).toHaveText('Example: 1.235M')
        await expect(page.getByRole('group', { name: 'Number display', exact: true })).toBeVisible()
        for (const button of [compact, full]) {
          const box = await button.boundingBox()
          expect(box?.height).toBeGreaterThanOrEqual(44)
        }
        const pay = page.getByTestId('swap-you-pay-amount')
        await pay.fill('1000.5')
        await full.click()
        await expect(preview).toHaveText('Example: 1,234,567')
        await expect(full).toHaveAttribute('aria-pressed', 'true')
        await expect(pay).toHaveValue('1000.5')
        expect(await page.evaluate((key) => localStorage.getItem(key), STORAGE_KEY)).toBe('full')
        for (const key of ['format', 'compact', 'locale']) {
          expect(new URL(page.url()).searchParams.has(key)).toBe(false)
        }
        const screenshot = testInfo.outputPath(`settings-full-${theme}-${width}.png`)
        await page.screenshot({ path: screenshot, fullPage: true })
        await testInfo.attach(`settings-full-${theme}-${width}`, {
          path: screenshot,
          contentType: 'image/png',
        })
        await page.reload()
        await openSettings(page)
        await expect(full).toHaveAttribute('aria-pressed', 'true')
        await expect(preview).toHaveText('Example: 1,234,567')
        await compact.click()
        await expect(preview).toHaveText('Example: 1.235M')
        await expect(compact).toHaveAttribute('aria-pressed', 'true')
      })
    }
  }

  test('hostile storage and query format keys cannot opt into Full', async ({ page }) => {
    await page.addInitScript((key) => localStorage.setItem(key, '<img src=x onerror=alert(1)>'), STORAGE_KEY)
    await page.goto('/?format=full&compact=0&locale=de-DE')
    await openSettings(page)
    await expect(page.getByTestId('swap-number-display-compact')).toHaveAttribute('aria-pressed', 'true')
    await expect(page.getByTestId('swap-number-display-preview')).toHaveText('Example: 1.235M')
    await expect(page.locator('#swap-slippage-settings img')).toHaveCount(0)
  })
})
