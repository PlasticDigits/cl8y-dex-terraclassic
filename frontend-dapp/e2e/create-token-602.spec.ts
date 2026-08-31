import { test, expect } from './fixtures/dev-wallet'

/**
 * GitLab #602 P402-4 / P402-5 — Create Token retail chrome + /create copy-address only.
 * e2e-smoke (5 workers). Does not submit on-chain txs.
 */
const QUERY_A = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const QUERY_B = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'

test.describe('Create Token post-merge QA (GitLab #602)', () => {
  test('P402-1: /token/create is Create Token, not the unavailable stub', async ({ page }) => {
    await page.goto('/token/create')
    await expect(page.getByTestId('create-token-page')).toBeVisible({ timeout: 20_000 })
    await expect(page.getByTestId('create-token-unavailable')).toHaveCount(0)
    await expect(page.getByRole('heading', { name: /create token/i })).toBeVisible()
    await expect(page.getByRole('heading', { name: /create pair/i })).toHaveCount(0)
  })

  test('P402-4: More menu lists Create Token (not Mint)', async ({ page }) => {
    await page.goto('/')
    await page
      .locator('header.app-header-shell nav.app-desktop-nav')
      .getByRole('button', { name: /^More$/i })
      .click()
    const createToken = page.getByRole('menuitem', { name: 'Create Token' })
    await expect(createToken).toBeVisible({ timeout: 15_000 })
    await expect(createToken).toHaveAttribute('href', '/token/create')
    await expect(page.getByRole('menuitem', { name: 'Mint' })).toHaveCount(0)
    await createToken.click()
    await expect(page.getByTestId('create-token-page')).toBeVisible({ timeout: 15_000 })
  })

  test('P402-4: paid SKU shows PayWithAnyToken; free path has Create Token CTA when connected', async ({
    page,
    connectWallet,
  }) => {
    await connectWallet
    await page.goto('/token/create')
    await expect(page.getByTestId('create-token-page')).toBeVisible({ timeout: 20_000 })
    await page.getByTestId('create-token-name').fill('QaToken')
    await page.getByTestId('create-token-symbol').fill('QATK')
    await page.getByTestId('create-token-ack').check()
    await expect(page.getByTestId('create-token-free-cta')).toBeVisible()
    await page.getByTestId('create-token-sku-transfer_tax').check()
    await expect(page.getByTestId('create-token-pay')).toBeVisible()
    await expect(page.getByTestId('create-token-pay-copy')).toContainText('50 UST1')
  })

  test('P402-5 / #713: /create?a=&b= prefills Token A/B and does not auto-submit', async ({ page }) => {
    await page.goto(`/create?a=${QUERY_A}&b=${QUERY_B}`)
    await expect(page.getByRole('heading', { name: /create trading pair/i })).toBeVisible({
      timeout: 20_000,
    })
    for (const side of ['token-a', 'token-b'] as const) {
      const toggle = page.getByTestId(`create-pair-custom-toggle-${side}`)
      if ((await toggle.getAttribute('aria-expanded')) !== 'true') {
        await toggle.click()
      }
    }
    await expect(page.getByTestId('create-pair-custom-address-token-a')).toHaveValue(QUERY_A)
    await expect(page.getByTestId('create-pair-custom-address-token-b')).toHaveValue(QUERY_B)
    await expect(page.getByRole('button', { name: /Create Pair/i })).toBeVisible()
  })

  test('P402-4: /tokens catalog page is reachable when Create Token is configured', async ({ page }) => {
    await page.goto('/tokens')
    await expect(page.locator('body')).toBeVisible()
    await expect(page.getByText(/create token is not configured/i)).toHaveCount(0)
  })
})

test.describe('Create Token desktop density (GitLab #669)', () => {
  test('1280×720: page wider than 700px; identity/wallets paired; Paid features in first viewport', async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 720 })
    await page.goto('/token/create')
    await expect(page.getByTestId('create-token-page')).toBeVisible({ timeout: 20_000 })

    const pageBox = await page.getByTestId('create-token-page').boundingBox()
    expect(pageBox?.width ?? 0).toBeGreaterThan(700)

    const name = await page.getByTestId('create-token-name').boundingBox()
    const symbol = await page.getByTestId('create-token-symbol').boundingBox()
    expect(Math.abs((name?.y ?? 0) - (symbol?.y ?? 0))).toBeLessThanOrEqual(48)

    const treasury = await page.getByTestId('create-token-treasury').boundingBox()
    const manager = await page.getByTestId('create-token-manager').boundingBox()
    expect(Math.abs((treasury?.y ?? 0) - (manager?.y ?? 0))).toBeLessThanOrEqual(48)

    const legend = await page.getByTestId('create-token-features-legend').boundingBox()
    expect(legend?.y ?? 9999).toBeLessThan(720)

    const ack = await page.getByTestId('create-token-ack').boundingBox()
    expect(ack?.y ?? 9999).toBeLessThan(720 * 2)
  })

  test('1440×900: width floor and does not stretch past .app-main', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await page.goto('/token/create')
    await expect(page.getByTestId('create-token-page')).toBeVisible({ timeout: 20_000 })
    const pageBox = await page.getByTestId('create-token-page').boundingBox()
    expect(pageBox?.width ?? 0).toBeGreaterThan(700)
    expect(pageBox?.width ?? 9999).toBeLessThanOrEqual(1080 + 24)
  })

  for (const size of [
    { width: 375, height: 667 },
    { width: 390, height: 844 },
  ] as const) {
    test(`${size.width}×${size.height}: stacked; no horizontal document scroll`, async ({ page }) => {
      await page.setViewportSize(size)
      await page.goto('/token/create')
      await expect(page.getByTestId('create-token-page')).toBeVisible({ timeout: 20_000 })

      const pageBox = await page.getByTestId('create-token-page').boundingBox()
      expect(pageBox?.width ?? 9999).toBeLessThanOrEqual(size.width - 16)

      const name = await page.getByTestId('create-token-name').boundingBox()
      const symbol = await page.getByTestId('create-token-symbol').boundingBox()
      const decimals = await page.getByTestId('create-token-decimals').boundingBox()
      expect(name?.y ?? 0).toBeLessThan(symbol?.y ?? 0)
      expect(symbol?.y ?? 0).toBeLessThan(decimals?.y ?? 0)

      await page
        .getByTestId('create-token-treasury')
        .fill('terra1yyca08xqdgvjz0psg56z67ejh9xms6l436u8y58m82npdqqhmmtqzjqhh0')
      const overflow = await page.evaluate(() => {
        const el = document.scrollingElement
        if (!el) return { scrollWidth: 0, clientWidth: 0 }
        return { scrollWidth: el.scrollWidth, clientWidth: el.clientWidth }
      })
      expect(overflow.scrollWidth).toBeLessThanOrEqual(overflow.clientWidth + 1)
    })
  }
})
