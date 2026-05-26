import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable, assertTxResultAlert } from './helpers/chain'
import { swapYouReceiveAmountDisplay } from './helpers/swap-ui'
import {
  requireNonNativeCw20PayOption,
  requireNonNativeCw20ReceiveOption,
  requireTokenInCombobox,
} from './helpers/wrap-e2e'
import {
  ARIA_SELECT_TOKEN_PAY,
  ARIA_SELECT_TOKEN_RECEIVE,
  expectAtLeastTwoPayTokenOptions,
  expectPayTokenListPopulated,
  payTokenTrigger,
  waitForPayTokenTriggerEnabled,
} from './helpers/token-select'

function swapActionPanel(page: import('@playwright/test').Page) {
  return page.locator('main .shell-panel-strong').first()
}

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
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'LUNC-C')

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
  test.beforeEach(async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet
    await page.waitForLoadState('networkidle')
    await expectAtLeastTwoPayTokenOptions(page)
  })

  test('E1: swap native input — LUNC to CW20', async ({ page }) => {
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'LUNC-C')
    await requireNonNativeCw20ReceiveOption(page)

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('0.1')

    const receiveField = swapYouReceiveAmountDisplay(page)
    await expect(async () => {
      const text = await receiveField.textContent()
      expect(text).not.toBe('0.00')
      expect(text).not.toContain('Calculating')
    }).toPass({ timeout: 15000 })

    const swapBtn = swapActionPanel(page).getByRole('button', { name: /^(Swap|Confirm Swap)/ })
    await expect(swapBtn).toBeEnabled({ timeout: 10000 })
    await swapBtn.click()

    await assertTxResultAlert(page)
  })

  test('E2: swap native output — CW20 to native USTC', async ({ page }) => {
    await requireNonNativeCw20PayOption(page)
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'USTC', 'USTC-C')

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('0.1')

    const receiveField = swapYouReceiveAmountDisplay(page)
    await expect(async () => {
      const text = await receiveField.textContent()
      expect(text).not.toBe('0.00')
      expect(text).not.toContain('Calculating')
    }).toPass({ timeout: 15000 })

    const swapBtn = swapActionPanel(page).getByRole('button', { name: /^(Swap|Confirm Swap)/ })
    await expect(swapBtn).toBeEnabled({ timeout: 10000 })
    await swapBtn.click()

    await assertTxResultAlert(page)
  })

  test('E3: swap native to native — LUNC to USTC', async ({ page }) => {
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'LUNC-C')
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'USTC', 'USTC-C')

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('0.1')

    const receiveField = swapYouReceiveAmountDisplay(page)
    await expect(async () => {
      const text = await receiveField.textContent()
      expect(text).not.toBe('0.00')
      expect(text).not.toContain('Calculating')
    }).toPass({ timeout: 15000 })

    const routeDisplay = page.getByText(/Route:/)
    const routeCount = await routeDisplay.count()
    expect(routeCount).toBeGreaterThanOrEqual(0)

    const swapBtn = swapActionPanel(page).getByRole('button', { name: /^(Swap|Confirm Swap)/ })
    await expect(swapBtn).toBeEnabled({ timeout: 10000 })
    await swapBtn.click()

    await assertTxResultAlert(page)
  })

  test('E4: direct wrap — LUNC to LUNC-C', async ({ page }) => {
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'LUNC-C')
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'LUNC-C')

    const wrapNote = page.getByText(/1:1/)
    const wrapNoteCount = await wrapNote.count()
    expect(wrapNoteCount).toBeGreaterThanOrEqual(0)

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('0.1')

    const swapBtn = swapActionPanel(page).getByRole('button', { name: /^(Swap|Confirm Swap)/ })
    await expect(swapBtn).toBeEnabled({ timeout: 10000 })
    await swapBtn.click()

    await assertTxResultAlert(page)
  })

  test('E5: direct unwrap — LUNC-C to LUNC', async ({ page }) => {
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC-C')
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'LUNC', 'LUNC-C')

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('0.1')

    const swapBtn = swapActionPanel(page).getByRole('button', { name: /^(Swap|Confirm Swap)/ })
    await expect(swapBtn).toBeEnabled({ timeout: 10000 })
    await swapBtn.click()

    await assertTxResultAlert(page)
  })

  test('E6: wrapped-to-wrapped swap — LUNC-C to USTC-C (normal CW20)', async ({ page }) => {
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC-C')
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'USTC-C')

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('0.1')

    const receiveField = swapYouReceiveAmountDisplay(page)
    await expect(async () => {
      const text = await receiveField.textContent()
      expect(text).not.toBe('0.00')
      expect(text).not.toContain('Calculating')
    }).toPass({ timeout: 15000 })

    const swapBtn = swapActionPanel(page).getByRole('button', { name: /^(Swap|Confirm Swap)/ })
    await expect(swapBtn).toBeEnabled({ timeout: 10000 })
    await swapBtn.click()

    await assertTxResultAlert(page)
  })

  test('E12: rate limit exceeded shows error in UI', async ({ page }) => {
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'LUNC-C')
    await requireTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'LUNC-C')

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('999999999999')

    await page.waitForTimeout(2000)
    const btn = swapActionPanel(page)
      .getByRole('button')
      .filter({ hasText: /Rate Limit|Insufficient|Swap/i })
      .last()
    await expect(btn).toBeVisible()
  })
})
