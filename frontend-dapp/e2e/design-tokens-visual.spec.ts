import { mkdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect } from '@playwright/test'

/**
 * Design token visual captures for GitLab #488 (blue+gold).
 * Run: cd frontend-dapp && PLAYWRIGHT_SKIP_CHAIN=1 npx playwright test e2e/design-tokens-visual.spec.ts --project=e2e-smoke
 */
const SCREENSHOT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'docs', 'qa', 'issue-488')

test.describe('Design token visual QA (GitLab #488)', () => {
  test.beforeAll(() => {
    mkdirSync(SCREENSHOT_DIR, { recursive: true })
  })

  test('trade bootstrap first paint uses cool blue on-theme skeleton', async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('cl8y-dex-theme', 'dark')
    })
    await page.goto('/bootstrap/trade-bootstrap-fixture.html', { waitUntil: 'domcontentloaded' })

    const bootstrap = page.locator('#trade-bootstrap-shell')
    await expect(bootstrap).toBeVisible({ timeout: 5_000 })

    const bg = await page.evaluate(() => {
      const bodyBg = getComputedStyle(document.body).backgroundColor
      const theme = document.documentElement.getAttribute('data-theme')
      return { bodyBg, theme }
    })

    expect(bg.theme).toBe('dark')
    // Cool navy #0d111c — not warm brown #0e0908
    expect(bg.bodyBg).not.toMatch(/rgb\(\s*14,\s*9,\s*8\s*\)/)
    expect(bg.bodyBg).toMatch(/rgb\(\s*13,\s*17,\s*28\s*\)/)

    await page.screenshot({
      path: join(SCREENSHOT_DIR, 'trade-bootstrap-first-paint.png'),
      fullPage: false,
    })
  })

  test('swap shell-panel light and dark themes', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 900 })

    await page.goto('/')
    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'dark')
      localStorage.setItem('cl8y-dex-theme', 'dark')
    })
    await page.reload({ waitUntil: 'networkidle' })
    const darkPanel = page.locator('.shell-panel, .shell-panel-strong').first()
    await expect(darkPanel).toBeVisible({ timeout: 30_000 })
    const darkBuffer = await darkPanel.screenshot()
    const darkPath = join(SCREENSHOT_DIR, 'swap-shell-panel-dark.png')
    await darkPanel.screenshot({ path: darkPath })

    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'light')
      localStorage.setItem('cl8y-dex-theme', 'light')
    })
    await page.reload({ waitUntil: 'networkidle' })
    const lightPanel = page.locator('.shell-panel, .shell-panel-strong').first()
    await expect(lightPanel).toBeVisible({ timeout: 30_000 })
    const lightBuffer = await lightPanel.screenshot()
    const lightPath = join(SCREENSHOT_DIR, 'swap-shell-panel-light.png')
    await lightPanel.screenshot({ path: lightPath })

    const darkB64 = darkBuffer.toString('base64')
    const lightB64 = lightBuffer.toString('base64')
    await page.setContent(`
      <style>body{margin:0;background:#666;display:flex;gap:12px;padding:12px;align-items:flex-start}</style>
      <img src="data:image/png;base64,${darkB64}" alt="dark shell-panel" style="max-width:48%" />
      <img src="data:image/png;base64,${lightB64}" alt="light shell-panel" style="max-width:48%" />
    `)
    await page.screenshot({
      path: join(SCREENSHOT_DIR, 'swap-shell-panel-light-dark-side-by-side.png'),
      fullPage: true,
    })
  })

  const primaryRoutes = [
    { path: '/', name: 'swap' },
    { path: '/trade', name: 'trade' },
    { path: '/limits', name: 'limits' },
    { path: '/pool', name: 'pool' },
    { path: '/portfolio', name: 'portfolio' },
    { path: '/charts', name: 'charts' },
    { path: '/trader', name: 'trader' },
    { path: '/protocol', name: 'protocol' },
  ] as const

  for (const route of primaryRoutes) {
    test(`primary route ${route.path} uses glass shell chrome`, async ({ page }) => {
      await page.goto(route.path, { waitUntil: 'domcontentloaded' })
      await page.evaluate(() => {
        document.documentElement.setAttribute('data-theme', 'dark')
        localStorage.setItem('cl8y-dex-theme', 'dark')
      })
      await expect(async () => {
        const shellCount = await page.locator('.shell-panel, .shell-panel-strong, #trade-bootstrap-shell').count()
        expect(shellCount).toBeGreaterThan(0)
      }).toPass({ timeout: 45_000 })
    })
  }
})
