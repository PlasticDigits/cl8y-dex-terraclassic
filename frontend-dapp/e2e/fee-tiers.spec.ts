import { test, expect } from './fixtures/dev-wallet'
import { clickDesktopMoreNavItem } from './helpers/desktop-more-nav'

test.describe('Fee Tiers Page', () => {
  test('shows Fee Discount Tiers heading', async ({ page }) => {
    await page.goto('/tiers')
    await expect(page.getByRole('heading', { name: /Fee Discount Tiers/i })).toBeVisible()
  })

  test('shows description about CL8Y tokens', async ({ page }) => {
    await page.goto('/tiers')
    await expect(page.getByText(/Hold the configured CL8Y CW20/i)).toBeVisible()
  })

  test('displays all 9 public fee tiers', async ({ page }) => {
    await page.goto('/tiers')
    await page.waitForLoadState('networkidle')

    for (let i = 1; i <= 9; i++) {
      await expect(page.getByText(`Tier ${i}`).first()).toBeVisible()
    }
  })

  test('shows discount percentages for each tier', async ({ page }) => {
    await page.goto('/tiers')
    await page.waitForLoadState('networkidle')

    await expect(page.getByText('2.5%').first()).toBeVisible()
    await expect(page.getByText('10%').first()).toBeVisible()
    await expect(page.getByText('20%').first()).toBeVisible()
    await expect(page.getByText('35%').first()).toBeVisible()
    await expect(page.getByText('50%').first()).toBeVisible()
    await expect(page.getByText('60%').first()).toBeVisible()
    await expect(page.getByText('75%').first()).toBeVisible()
    await expect(page.getByText('85%').first()).toBeVisible()
    await expect(page.getByText('95%').first()).toBeVisible()
  })

  test('shows CL8Y holding requirements', async ({ page }) => {
    await page.goto('/tiers')
    await page.waitForLoadState('networkidle')

    await expect(page.getByText(/Hold.*CL8Y/i).first()).toBeVisible()
  })

  test('shows How It Works section', async ({ page }) => {
    await page.goto('/tiers')
    await expect(page.getByRole('heading', { name: /How it works/i })).toBeVisible()
  })

  test('shows CL8Y hold requirements in How It Works table', async ({ page }) => {
    await page.goto('/tiers')
    await page.waitForLoadState('networkidle')

    const howItWorks = page.getByRole('heading', { name: /How it works/i })
    await howItWorks.scrollIntoViewIfNeeded()
    await expect(page.getByTestId('tiers-how-it-works-table').getByText('CL8Y Hold', { exact: true })).toBeVisible({
      timeout: 15000,
    })
    await expect(page.getByText(/drop below.*lose your tier/i)).toBeVisible()
  })

  test('shows CL8Y hold amounts per tier', async ({ page }) => {
    await page.goto('/tiers')
    await page.waitForLoadState('networkidle')

    await expect(async () => {
      await expect(page.getByText(/Hold.*CL8Y/i).first()).toBeVisible()
    }).toPass({ timeout: 15000 })
  })

  test('shows effective fee column in How It Works', async ({ page }) => {
    await page.goto('/tiers')
    await page.waitForLoadState('networkidle')

    const howItWorks = page.getByRole('heading', { name: /How it works/i })
    await howItWorks.scrollIntoViewIfNeeded()
    await expect(page.getByText('Eff. Fee*', { exact: true }).first()).toBeVisible({ timeout: 15000 })
    await expect(page.getByText(/default base fee is 1\.8/i)).toBeVisible()
  })

  test('prompts to connect wallet for registration', async ({ page }) => {
    await page.goto('/tiers')
    await page.waitForLoadState('networkidle')

    await expect(page.getByText(/Connect your wallet/i)).toBeVisible()
  })

  test.describe('With wallet connected', () => {
    test('shows register buttons for tiers', async ({ page, connectWallet }) => {
      await connectWallet
      await clickDesktopMoreNavItem(page, 'Fee Tiers')
      await page.waitForURL('/tiers')
      await page.waitForLoadState('networkidle')

      await expect(async () => {
        const registerBtns = page.getByRole('button', { name: /Register/i })
        const count = await registerBtns.count()
        expect(count).toBeGreaterThanOrEqual(1)
      }).toPass({ timeout: 10000 })
    })
  })

  test.describe('Phone-width layout (#651)', () => {
    const PHONES = [
      { width: 390, height: 844 },
      { width: 375, height: 667 },
    ] as const

    for (const size of PHONES) {
      test(`disconnected ${size.width}×${size.height}: Hold phrases + no empty Register hole`, async ({ page }) => {
        await page.setViewportSize(size)
        await page.goto('/tiers')
        await page.waitForLoadState('networkidle')

        await expect(page.getByTestId('tier-hold-1')).toHaveText(/^Hold 1 CL8Y$/)
        await expect(page.getByTestId('tier-hold-2')).toHaveText(/^Hold 5 CL8Y$/)
        await expect(page.getByTestId('tier-hold-9')).toHaveText(/^Hold 7\.5K CL8Y$/)

        for (const id of [1, 2, 9] as const) {
          const hold = page.getByTestId(`tier-hold-${id}`)
          const box = await hold.boundingBox()
          expect(box, `hold ${id} box`).toBeTruthy()
          expect(box!.height).toBeLessThanOrEqual(28)

          const feeDiscount = page.getByTestId(`tier-fee-cluster-${id}`).getByText(/^fee discount$/i)
          const feeBox = await feeDiscount.boundingBox()
          expect(feeBox, `fee discount ${id} box`).toBeTruthy()
          expect(feeBox!.height).toBeLessThanOrEqual(22)

          const card = page.getByTestId(`tier-card-${id}`)
          const cardBox = await card.boundingBox()
          const clusterBox = await page.getByTestId(`tier-fee-cluster-${id}`).boundingBox()
          expect(cardBox).toBeTruthy()
          expect(clusterBox).toBeTruthy()
          const trailingHole = cardBox!.x + cardBox!.width - (clusterBox!.x + clusterBox!.width)
          expect(trailingHole).toBeLessThan(32)
        }

        await expect(page.getByRole('button', { name: /^Register$/i })).toHaveCount(0)
        await expect(page.getByTestId('tiers-how-it-works-mobile')).toBeVisible()
        await expect(page.getByTestId('tiers-how-it-works-table')).toBeHidden()
        await expect(
          page.getByTestId('how-it-works-row-tier-9').getByText('Limit place*', { exact: true })
        ).toBeVisible()
        await expect(page.getByTestId('how-it-works-row-tier-9').getByText('0%', { exact: true })).toBeVisible()
      })
    }

    test('connected 390×844: Register ≥44px and does not overlap the previous card', async ({
      page,
      connectWallet,
    }) => {
      await page.setViewportSize({ width: 390, height: 844 })
      await connectWallet
      await page.goto('/tiers')
      await page.waitForLoadState('networkidle')

      await expect(async () => {
        expect(await page.getByTestId(/^register-tier-/).count()).toBeGreaterThanOrEqual(1)
      }).toPass({ timeout: 10_000 })

      const buttons = page.getByTestId(/^register-tier-/)
      const n = await buttons.count()
      expect(n).toBeGreaterThanOrEqual(1)

      for (let i = 0; i < n; i++) {
        const btn = buttons.nth(i)
        const box = await btn.boundingBox()
        expect(box, `register ${i}`).toBeTruthy()
        expect(box!.height).toBeGreaterThanOrEqual(44)
      }

      const mid = page.getByTestId('register-tier-5')
      if (await mid.isVisible().catch(() => false)) {
        const holdPrev = await page.getByTestId('tier-hold-4').boundingBox()
        const midBox = await mid.boundingBox()
        expect(holdPrev).toBeTruthy()
        expect(midBox).toBeTruthy()
        expect(midBox!.y).toBeGreaterThanOrEqual(holdPrev!.y + holdPrev!.height)
      }
    })
  })
})
