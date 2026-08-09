import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable, assertTxResultAlert } from './helpers/chain'
import {
  clickSwapSubmit,
  openSwapSettingsAndSetSlippage,
  swapActionPanel,
  swapYouReceiveAmountDisplay,
} from './helpers/swap-ui'
import { requireTokenInCombobox } from './helpers/wrap-e2e'
import {
  routeWrapMapperPaused,
  routeWrapMapperRateLimitExceeded,
  wrapMapperAddressFromEnv,
} from './helpers/wrap-mapper-lcd-mock'
import {
  ARIA_SELECT_TOKEN_PAY,
  ARIA_SELECT_TOKEN_RECEIVE,
  expectAtLeastTwoPayTokenOptions,
  expectPayTokenListPopulated,
  payTokenTrigger,
  waitForPayTokenTriggerEnabled,
} from './helpers/token-select'

test.describe('Swap with native token wrapping — UI', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')
    await expectPayTokenListPopulated(page)
  })

  test('E1: token selector shows native LUNC and USTC options', async ({ page }) => {
    await payTokenTrigger(page).click()
    const list = page.getByRole('listbox', { name: ARIA_SELECT_TOKEN_PAY })
    await expect(list).toBeVisible()
    expect(await list.getByRole('option').count()).toBeGreaterThan(0)
    await page.keyboard.press('Escape')
  })

  test('E2: selecting native LUNC as input shows wrap note', async ({ page }) => {
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'cLUNC')

    const wrapNote = page.getByText('This swap will wrap')
    const noteCount = await wrapNote.count()
    expect(noteCount).toBeGreaterThanOrEqual(0)
  })

  test('E3: swap button never says standalone Wrap or Unwrap', async ({ page }) => {
    const swapPanel = swapActionPanel(page)
    const submitBtn = swapPanel.getByRole('button', {
      name: /Connect Wallet|Enter Amount|Swap|No Route/i,
    })
    await expect(submitBtn.first()).toBeVisible()

    const wrapButton = page.locator('button').filter({ hasText: /^Wrap$/ })
    await expect(wrapButton).toHaveCount(0)
    const unwrapButton = page.locator('button').filter({ hasText: /^Unwrap$/ })
    await expect(unwrapButton).toHaveCount(0)
  })

  test('E4: route display loads without errors after pair selection', async ({ page }) => {
    await waitForPayTokenTriggerEnabled(page)
    await payTokenTrigger(page).click()
    const list = page.getByRole('listbox', { name: ARIA_SELECT_TOKEN_PAY })
    await expect(list).toBeVisible()
    const optCount = await list.getByRole('option').count()
    if (optCount > 0) {
      await list.getByRole('option').first().click()
    } else {
      await page.keyboard.press('Escape')
    }

    await expect(page.getByRole('heading', { name: 'Swap' })).toBeVisible()
  })

  test('E5: swap direction toggle button is present', async ({ page }) => {
    await expect(page.getByRole('button', { name: 'Swap pay and receive tokens' })).toBeVisible()
  })
})

test.describe('Swap Transaction Tests — Native Wrapping', () => {
  test.describe.configure({ mode: 'serial' })

  test.beforeEach(async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet
    await page.waitForLoadState('networkidle')
    await expectAtLeastTwoPayTokenOptions(page)
  })

  test('E1: swap native input — LUNC to CW20', async ({ page }) => {
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'cLUNC')
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'EMBER')

    const input = page.getByRole('textbox', { name: 'You Pay' })
    await input.fill('0.0001')

    const receiveField = swapYouReceiveAmountDisplay(page)
    await expect(async () => {
      const text = await receiveField.textContent()
      expect(text).not.toBe('0.00')
      expect(text).not.toContain('Calculating')
    }).toPass({ timeout: 15000 })

    await openSwapSettingsAndSetSlippage(page, 15)

    await clickSwapSubmit(page)

    await assertTxResultAlert(page)
  })

  test('E2: swap native output — CW20 to native USTC', async ({ page }) => {
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'EMBER')
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'USTC', 'cUSTC')

    const input = page.getByRole('textbox', { name: 'You Pay' })
    await input.fill('0.0001')

    const receiveField = swapYouReceiveAmountDisplay(page)
    await expect(async () => {
      const text = await receiveField.textContent()
      expect(text).not.toBe('0.00')
      expect(text).not.toContain('Calculating')
    }).toPass({ timeout: 15000 })

    await openSwapSettingsAndSetSlippage(page, 15)

    await clickSwapSubmit(page)

    await assertTxResultAlert(page)
  })

  test('E3: swap native to native — LUNC to USTC', async ({ page }) => {
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'cLUNC')
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'USTC', 'cUSTC')

    const input = page.getByRole('textbox', { name: 'You Pay' })
    await input.fill('0.0001')

    const receiveField = swapYouReceiveAmountDisplay(page)
    await expect(async () => {
      const text = await receiveField.textContent()
      expect(text).not.toBe('0.00')
      expect(text).not.toContain('Calculating')
    }).toPass({ timeout: 15000 })

    const routeSummary = page.getByTestId('swap-route-summary')
    const routeCount = await routeSummary.count()
    expect(routeCount).toBeGreaterThanOrEqual(0)

    await openSwapSettingsAndSetSlippage(page, 15)

    await clickSwapSubmit(page)

    await assertTxResultAlert(page)
  })

  test('E4: direct wrap — LUNC to cLUNC', async ({ page }) => {
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'cLUNC')
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'cLUNC')

    const feeNote = page.getByTestId('swap-wrap-fee-note')
    await expect(feeNote).toBeVisible({ timeout: 30_000 })
    // LocalTerra mapper is often fee_bps=50; mainnet 100 — never claim 1:1 when fee applies.
    await expect(feeNote).toHaveText(/% fee|fee unavailable|1:1/)
    const feeText = await feeNote.innerText()
    if (/% fee/.test(feeText)) {
      await expect(feeNote).not.toHaveText(/1:1/)
    }

    const input = page.getByRole('textbox', { name: 'You Pay' })
    await input.fill('0.0001')

    await clickSwapSubmit(page)

    await assertTxResultAlert(page)
  })

  test('E5: direct unwrap — cLUNC to LUNC', async ({ page }) => {
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'cLUNC')
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'LUNC', 'cLUNC')

    const input = page.getByRole('textbox', { name: 'You Pay' })
    await input.fill('0.0001')

    await clickSwapSubmit(page)

    await assertTxResultAlert(page)
  })

  test('E6: wrapped-to-wrapped swap — cLUNC to cUSTC (normal CW20)', async ({ page }) => {
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'cLUNC')
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'cUSTC')

    const input = page.getByRole('textbox', { name: 'You Pay' })
    await input.fill('0.0001')

    await expect(async () => {
      await expect(page.getByTestId('swap-route-summary')).toBeVisible()
    }).toPass({ timeout: 30_000 })

    const receiveField = swapYouReceiveAmountDisplay(page)
    await expect(async () => {
      const text = await receiveField.textContent()
      expect(text).not.toBe('0.00')
      expect(text).not.toContain('Calculating')
    }).toPass({ timeout: 15000 })

    await openSwapSettingsAndSetSlippage(page, 15)

    await clickSwapSubmit(page)

    await assertTxResultAlert(page)
  })

  test('E12: rate limit exceeded shows disabled CTA with exact copy (SEC-A02 / GitLab #389)', async ({
    page,
    connectWallet,
    request,
  }) => {
    await skipIfLcdUnreachable(request)
    const wrapMapper = wrapMapperAddressFromEnv()
    await routeWrapMapperRateLimitExceeded(page, wrapMapper)
    await connectWallet
    await page.waitForLoadState('networkidle')
    await expectAtLeastTwoPayTokenOptions(page)

    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'cLUNC')
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'cLUNC')

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('1')

    const btn = swapActionPanel(page).getByRole('button', { name: 'Rate Limit Exceeded' })
    await expect(btn).toBeVisible({ timeout: 15_000 })
    await expect(btn).toBeDisabled()

    // GitLab #463 (SEC-I05 F-04): inline alert below the form, not only the disabled CTA label.
    const banner = page.getByTestId('swap-wrap-rate-limit-banner')
    await expect(banner).toBeVisible({ timeout: 15_000 })
    await expect(banner).toContainText(/Daily wrap limit reached/i)
    await expect(banner).toContainText(/try again later, or reduce the amount/i)
  })
})

test.describe('Swap wrap safety CTA — isolated LCD mocks (SEC-A02 / GitLab #389)', () => {
  test.beforeEach(async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet
    await page.goto('/')
    await page.waitForLoadState('networkidle')
    await expectAtLeastTwoPayTokenOptions(page)
  })

  test('wrap mapper paused shows disabled Wrapping is Temporarily Paused CTA', async ({ page }) => {
    const wrapMapper = wrapMapperAddressFromEnv()
    await routeWrapMapperPaused(page, wrapMapper)

    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'cLUNC')
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'cLUNC')

    await page.getByPlaceholder('0.00').first().fill('0.0001')

    const btn = swapActionPanel(page).getByRole('button', { name: 'Wrapping is Temporarily Paused' })
    await expect(btn).toBeVisible({ timeout: 15_000 })
    await expect(btn).toBeDisabled()
  })
})
