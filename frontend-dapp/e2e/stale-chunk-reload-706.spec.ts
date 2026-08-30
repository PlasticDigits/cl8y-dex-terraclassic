import type { Page, Request } from '@playwright/test'
import { expect, test } from './fixtures/dev-wallet'

/**
 * GitLab #706 — stale hashed lazy chunks after a Coolify-style roll.
 * Route-mock 404s Vite's PoolPage module (dev) / hashed assets (prod preview).
 * Does not depend on a live Coolify deploy. Workers: e2e-smoke default (5).
 */

const POOL_CHUNK = '**/src/pages/PoolPage*'
const POOL_HASHED = '**/assets/PoolPage*'

async function fulfillPoolChunk404(route: {
  fulfill: (opts: { status: number; contentType: string; body: string }) => Promise<void>
}) {
  await route.fulfill({ status: 404, contentType: 'text/plain', body: 'missing-chunk' })
}

async function clickPoolNav(page: Page) {
  await page.setViewportSize({ width: 1280, height: 800 })
  await page
    .locator('header.app-header-shell nav.app-desktop-nav')
    .getByRole('link', { name: /^Pool$/ })
    .click()
}

function trackDocumentRequests(page: Page) {
  let count = 0
  const onRequest = (req: Request) => {
    if (req.resourceType() === 'document') count += 1
  }
  page.on('request', onRequest)
  return {
    get count() {
      return count
    },
  }
}

test.describe('Stale lazy chunk reload (GitLab #706)', () => {
  test('first PoolPage 404 recovers with at most one extra document load', async ({ page }) => {
    let chunkHits = 0
    const blockFirst = async (route: Parameters<typeof fulfillPoolChunk404>[0] & { continue: () => Promise<void> }) => {
      chunkHits += 1
      if (chunkHits === 1) {
        await fulfillPoolChunk404(route)
        return
      }
      await route.continue()
    }
    await page.route(POOL_CHUNK, blockFirst)
    await page.route(POOL_HASHED, blockFirst)

    await page.goto('/')
    await page.waitForLoadState('domcontentloaded')
    await expect(page.locator('header.app-header-shell')).toBeVisible()

    const docs = trackDocumentRequests(page)
    await clickPoolNav(page)
    await expect(page).toHaveURL(/\/pool/)

    await expect(page.getByTestId('route-error-boundary')).toHaveCount(0, { timeout: 20_000 })
    await expect(page.locator('header.app-header-shell')).toBeVisible()
    expect(docs.count, 'at most one extra document reload').toBeLessThanOrEqual(1)
  })

  test('broken chunk after reload guard shows Reload app and does not loop', async ({ page }) => {
    await page.addInitScript(() => {
      sessionStorage.setItem('cl8y-dex-stale-chunk-reload', '1')
    })
    await page.route(POOL_CHUNK, (route) => fulfillPoolChunk404(route))
    await page.route(POOL_HASHED, (route) => fulfillPoolChunk404(route))

    await page.goto('/pool')
    await page.waitForLoadState('domcontentloaded')

    await expect(page.getByTestId('route-error-boundary')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('route-error-reload-app')).toBeVisible()
    await expect(page.getByRole('button', { name: /try again/i })).toBeVisible()
    await expect(page.locator('header.app-header-shell')).toBeVisible()

    const docs = trackDocumentRequests(page)
    await page.waitForTimeout(1500)
    expect(docs.count, 'guarded fallback must not document-reload-loop').toBe(0)
    await expect(page.getByTestId('route-error-boundary')).toBeVisible()
  })

  test('offline chunk miss does not auto-reload; Try Again remains (#172)', async ({ page, context }) => {
    await page.addInitScript(() => {
      Object.defineProperty(navigator, 'onLine', { configurable: true, get: () => false })
    })
    await page.goto('/')
    await page.waitForLoadState('domcontentloaded')
    await context.setOffline(true)

    await page.route(POOL_CHUNK, (route) => fulfillPoolChunk404(route))
    await page.route(POOL_HASHED, (route) => fulfillPoolChunk404(route))

    const docs = trackDocumentRequests(page)
    await clickPoolNav(page)
    await expect(page.getByTestId('route-error-boundary')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByRole('button', { name: /try again/i })).toBeVisible()
    expect(docs.count, 'offline must not document-reload').toBe(0)

    await context.setOffline(false)
  })
})
