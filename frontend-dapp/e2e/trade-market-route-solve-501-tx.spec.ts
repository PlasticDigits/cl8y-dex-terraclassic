import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable } from './helpers/chain'
import { requireDualCwPair } from './helpers/hybrid-e2e'
import { gotoAndCaptureFactoryPairsPage } from './helpers/lcd'

/**
 * GitLab #501 — `/trade` Market defaults to indexer GET `/route/solve`
 * (same best-execution path as Swap). Advanced typed book → POST.
 */
test.describe('Trade market GET /route/solve default (GitLab #501)', () => {
  test('default Market quote uses GET /api/v1/route/solve (not POST)', async ({ page, connectWallet, request }) => {
    test.setTimeout(180_000)
    await skipIfLcdUnreachable(request)
    await connectWallet

    const pairs = await gotoAndCaptureFactoryPairsPage(page, '/')
    const { pair } = requireDualCwPair(pairs)

    const getSolves: string[] = []
    const postSolves: string[] = []
    page.on('request', (req) => {
      const url = req.url()
      if (!url.includes('/api/v1/route/solve')) return
      if (req.method() === 'GET') getSolves.push(url)
      if (req.method() === 'POST') postSolves.push(url)
    })

    await page.goto(`/trade/${pair.contract_addr}`)
    await page.waitForLoadState('networkidle')
    await page.getByTestId('trade-order-tab-market').click()

    await page.getByTestId('limit-order-escrow-amount-input').fill('1')
    await expect(page.getByTestId('trade-market-quote')).toBeVisible({ timeout: 90_000 })

    await expect.poll(() => getSolves.length, { timeout: 60_000 }).toBeGreaterThan(0)
    expect(postSolves, 'default empty book must not POST /route/solve').toHaveLength(0)
    await expect(page.getByTestId('trade-market-route-summary')).toBeVisible()
  })

  test('Advanced typed book leg uses POST /api/v1/route/solve', async ({ page, connectWallet, request }) => {
    test.setTimeout(180_000)
    await skipIfLcdUnreachable(request)
    await connectWallet

    const pairs = await gotoAndCaptureFactoryPairsPage(page, '/')
    const { pair } = requireDualCwPair(pairs)

    let sawPost = false
    page.on('request', (req) => {
      if (req.method() === 'POST' && req.url().includes('/api/v1/route/solve')) {
        sawPost = true
      }
    })

    await page.goto(`/trade/${pair.contract_addr}`)
    await page.waitForLoadState('networkidle')
    await page.getByTestId('trade-order-tab-market').click()
    await page.getByTestId('trade-market-advanced-toggle').click()
    await page.getByTestId('limit-order-escrow-amount-input').fill('1')
    await page.getByTestId('trade-market-book-leg-input').fill('0.25')

    await expect(page.getByTestId('trade-market-quote')).toBeVisible({ timeout: 90_000 })
    await expect.poll(() => sawPost, { timeout: 60_000 }).toBe(true)
  })
})
