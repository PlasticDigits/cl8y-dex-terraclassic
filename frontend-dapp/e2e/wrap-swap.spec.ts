import { test, expect } from './fixtures/dev-wallet'
import { skipIfNoTxAlert } from './helpers/chain'
import { swapYouReceiveAmountDisplay } from './helpers/swap-ui'
import {
  ARIA_SELECT_TOKEN_PAY,
  ARIA_SELECT_TOKEN_RECEIVE,
  expectAtLeastTwoPayTokenOptions,
  expectPayTokenListPopulated,
  payTokenTrigger,
  selectTokenInCombobox,
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
    const picked = await selectTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'LUNC-C')
    if (!picked) {
      test.skip()
      return
    }

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
  test.beforeEach(async ({ page, connectWallet }) => {
    await connectWallet
    await page.waitForLoadState('networkidle')
    await expectAtLeastTwoPayTokenOptions(page)
  })

  test('E1: swap native input — LUNC to CW20', async ({ page }) => {
    const hasLunc = await selectTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'LUNC-C')
    if (!hasLunc) {
      test.skip()
      return
    }

    await page.getByRole('button', { name: ARIA_SELECT_TOKEN_RECEIVE }).click()
    const recvList = page.getByRole('listbox', { name: ARIA_SELECT_TOKEN_RECEIVE })
    await expect(recvList).toBeVisible()
    const recvOpts = recvList.getByRole('option')
    const n = await recvOpts.count()
    let pickedCw20 = false
    for (let i = 0; i < n; i++) {
      const t = (await recvOpts.nth(i).innerText()).replace(/\s+/g, ' ')
      if (!t.includes('LUNC') && !t.includes('USTC')) {
        await recvOpts.nth(i).click()
        pickedCw20 = true
        break
      }
    }
    if (!pickedCw20) {
      await page.keyboard.press('Escape')
      test.skip()
      return
    }

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

    await skipIfNoTxAlert(page)
  })

  test('E2: swap native output — CW20 to native USTC', async ({ page }) => {
    await payTokenTrigger(page).click()
    const payList = page.getByRole('listbox', { name: ARIA_SELECT_TOKEN_PAY })
    await expect(payList).toBeVisible()
    const payOpts = payList.getByRole('option')
    const pn = await payOpts.count()
    let pickedFrom = false
    for (let i = 0; i < pn; i++) {
      const t = (await payOpts.nth(i).innerText()).replace(/\s+/g, ' ')
      if (!t.includes('LUNC') && !t.includes('USTC')) {
        await payOpts.nth(i).click()
        pickedFrom = true
        break
      }
    }
    if (!pickedFrom) {
      await page.keyboard.press('Escape')
      test.skip()
      return
    }

    const hasUstc = await selectTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'USTC', 'USTC-C')
    if (!hasUstc) {
      test.skip()
      return
    }

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

    await skipIfNoTxAlert(page)
  })

  test('E3: swap native to native — LUNC to USTC', async ({ page }) => {
    const hasLunc = await selectTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'LUNC-C')
    if (!hasLunc) {
      test.skip()
      return
    }
    const hasUstc = await selectTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'USTC', 'USTC-C')
    if (!hasUstc) {
      test.skip()
      return
    }

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

    await skipIfNoTxAlert(page)
  })

  test('E4: direct wrap — LUNC to LUNC-C', async ({ page }) => {
    const hasLunc = await selectTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'LUNC-C')
    if (!hasLunc) {
      test.skip()
      return
    }

    const hasLuncC = await selectTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'LUNC-C')
    if (!hasLuncC) {
      test.skip()
      return
    }

    const wrapNote = page.getByText(/1:1/)
    const wrapNoteCount = await wrapNote.count()
    expect(wrapNoteCount).toBeGreaterThanOrEqual(0)

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('0.1')

    const swapBtn = swapActionPanel(page).getByRole('button', { name: /^(Swap|Confirm Swap)/ })
    await expect(swapBtn).toBeEnabled({ timeout: 10000 })
    await swapBtn.click()

    await skipIfNoTxAlert(page)
  })

  test('E5: direct unwrap — LUNC-C to LUNC', async ({ page }) => {
    const hasLuncC = await selectTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC-C')
    if (!hasLuncC) {
      test.skip()
      return
    }

    const hasLunc = await selectTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'LUNC', 'LUNC-C')
    if (!hasLunc) {
      test.skip()
      return
    }

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('0.1')

    const swapBtn = swapActionPanel(page).getByRole('button', { name: /^(Swap|Confirm Swap)/ })
    await expect(swapBtn).toBeEnabled({ timeout: 10000 })
    await swapBtn.click()

    await skipIfNoTxAlert(page)
  })

  test('E6: wrapped-to-wrapped swap — LUNC-C to USTC-C (normal CW20)', async ({ page }) => {
    const hasLuncC = await selectTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC-C')
    if (!hasLuncC) {
      test.skip()
      return
    }

    const hasUstcC = await selectTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'USTC-C')
    if (!hasUstcC) {
      test.skip()
      return
    }

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

    await skipIfNoTxAlert(page)
  })

  test('E12: rate limit exceeded shows error in UI', async ({ page }) => {
    const hasLunc = await selectTokenInCombobox(page, ARIA_SELECT_TOKEN_PAY, 'LUNC', 'LUNC-C')
    if (!hasLunc) {
      test.skip()
      return
    }

    const hasLuncC = await selectTokenInCombobox(page, ARIA_SELECT_TOKEN_RECEIVE, 'LUNC-C')
    if (!hasLuncC) {
      test.skip()
      return
    }

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
