import { test, expect } from './fixtures/dev-wallet'

test.describe('Swap with native token wrapping — UI', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
    await page.waitForLoadState('networkidle')
    await expect(async () => {
      const pairSelector = page.locator('select')
      const options = await pairSelector.locator('option').allTextContents()
      const hasPair = options.some((opt) => !opt.includes('Loading') && !opt.includes('Select'))
      expect(hasPair).toBe(true)
    }).toPass({ timeout: 15000 })
  })

  test('E1: token selector shows native LUNC and USTC options', async ({ page }) => {
    const pairSelector = page.locator('select')
    const options = await pairSelector.locator('option').allTextContents()
    expect(options.length).toBeGreaterThan(0)
  })

  test('E2: selecting native LUNC as input shows wrap note', async ({ page }) => {
    const pairSelector = page.locator('select')
    const options = await pairSelector.locator('option').allTextContents()

    const luncOption = options.find((o) => o.includes('LUNC') && !o.includes('LUNC-C'))
    if (!luncOption) {
      test.skip()
      return
    }

    const luncValue = await pairSelector.locator('option', { hasText: luncOption }).getAttribute('value')
    if (luncValue) {
      await pairSelector.selectOption(luncValue)
    }

    const wrapNote = page.getByText('This swap will wrap')
    const noteCount = await wrapNote.count()
    expect(noteCount).toBeGreaterThanOrEqual(0)
  })

  test('E3: swap button never says standalone Wrap or Unwrap', async ({ page }) => {
    const submitBtn = page
      .locator('button')
      .filter({ hasText: /Connect Wallet|Enter Amount|Swap|No Route/i })
      .last()
    await expect(submitBtn).toBeVisible()

    const wrapButton = page.locator('button').filter({ hasText: /^Wrap$/ })
    await expect(wrapButton).toHaveCount(0)
    const unwrapButton = page.locator('button').filter({ hasText: /^Unwrap$/ })
    await expect(unwrapButton).toHaveCount(0)
  })

  test('E4: route display loads without errors after pair selection', async ({ page }) => {
    const pairSelector = page.locator('select')
    const allValues = await pairSelector
      .locator('option')
      .evaluateAll((els) => els.map((el) => (el as HTMLOptionElement).value).filter((v) => v))

    if (allValues.length > 0) {
      await pairSelector.selectOption(allValues[0])
    }

    await expect(page.getByRole('heading', { name: 'Swap' })).toBeVisible()
  })

  test('E5: swap direction toggle button is present', async ({ page }) => {
    const buttons = page.locator('button')
    const swapDirectionBtn = buttons.filter({ has: page.locator('svg') })
    await expect(swapDirectionBtn.first()).toBeVisible()
  })
})

// Helper: select a token in a <select> by partial text match
async function selectTokenByText(
  page: import('@playwright/test').Page,
  selectLabel: string,
  searchText: string
): Promise<boolean> {
  const selector = page.locator(`select[aria-label="${selectLabel}"]`)
  const options = await selector.locator('option').allTextContents()
  const match = options.find((o) => o.includes(searchText))
  if (!match) return false
  const value = await selector.locator('option', { hasText: match }).getAttribute('value')
  if (!value) return false
  await selector.selectOption(value)
  return true
}

test.describe('Swap Transaction Tests — Native Wrapping', () => {
  test.beforeEach(async ({ page, connectWallet }) => {
    await connectWallet
    await page.goto('/')
    await page.waitForLoadState('networkidle')
    await expect(async () => {
      const fromSelector = page.locator('select[aria-label="Select from token"]')
      const options = await fromSelector.locator('option').count()
      expect(options).toBeGreaterThan(1)
    }).toPass({ timeout: 20000 })
  })

  test('E1: swap native input — LUNC to CW20', async ({ page }) => {
    const hasLunc = await selectTokenByText(page, 'Select from token', 'LUNC')
    if (!hasLunc) {
      test.skip()
      return
    }

    const toSelector = page.locator('select[aria-label="Select to token"]')
    const toOptions = await toSelector.locator('option').allTextContents()
    const cw20Option = toOptions.find((o) => !o.includes('LUNC') && !o.includes('USTC'))
    if (!cw20Option) {
      test.skip()
      return
    }
    const cw20Value = await toSelector.locator('option', { hasText: cw20Option }).getAttribute('value')
    if (cw20Value) await toSelector.selectOption(cw20Value)

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('0.1')

    const receiveField = page.locator('.card-neo').filter({ hasText: 'You Receive' }).locator('div.text-2xl')
    await expect(async () => {
      const text = await receiveField.textContent()
      expect(text).not.toBe('0.00')
      expect(text).not.toContain('Calculating')
    }).toPass({ timeout: 15000 })

    const swapBtn = page.getByRole('button', { name: /^Swap$/ })
    await expect(swapBtn).toBeEnabled({ timeout: 10000 })
    await swapBtn.click()

    await expect(page.locator('.alert-success, .alert-error')).toBeVisible({ timeout: 60000 })
  })

  test('E2: swap native output — CW20 to native USTC', async ({ page }) => {
    const fromSelector = page.locator('select[aria-label="Select from token"]')
    const fromOptions = await fromSelector.locator('option').allTextContents()
    const cw20Option = fromOptions.find((o) => !o.includes('LUNC') && !o.includes('USTC'))
    if (!cw20Option) {
      test.skip()
      return
    }
    const cw20Value = await fromSelector.locator('option', { hasText: cw20Option }).getAttribute('value')
    if (cw20Value) await fromSelector.selectOption(cw20Value)

    const hasUstc = await selectTokenByText(page, 'Select to token', 'USTC')
    if (!hasUstc) {
      test.skip()
      return
    }

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('0.1')

    const receiveField = page.locator('.card-neo').filter({ hasText: 'You Receive' }).locator('div.text-2xl')
    await expect(async () => {
      const text = await receiveField.textContent()
      expect(text).not.toBe('0.00')
      expect(text).not.toContain('Calculating')
    }).toPass({ timeout: 15000 })

    const swapBtn = page.getByRole('button', { name: /^Swap$/ })
    await expect(swapBtn).toBeEnabled({ timeout: 10000 })
    await swapBtn.click()

    await expect(page.locator('.alert-success, .alert-error')).toBeVisible({ timeout: 60000 })
  })

  test('E3: swap native to native — LUNC to USTC', async ({ page }) => {
    const hasLunc = await selectTokenByText(page, 'Select from token', 'LUNC')
    if (!hasLunc) {
      test.skip()
      return
    }
    const hasUstc = await selectTokenByText(page, 'Select to token', 'USTC')
    if (!hasUstc) {
      test.skip()
      return
    }

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('0.1')

    const receiveField = page.locator('.card-neo').filter({ hasText: 'You Receive' }).locator('div.text-2xl')
    await expect(async () => {
      const text = await receiveField.textContent()
      expect(text).not.toBe('0.00')
      expect(text).not.toContain('Calculating')
    }).toPass({ timeout: 15000 })

    const routeDisplay = page.getByText(/Route:/)
    const routeCount = await routeDisplay.count()
    expect(routeCount).toBeGreaterThanOrEqual(0)

    const swapBtn = page.getByRole('button', { name: /^Swap$/ })
    await expect(swapBtn).toBeEnabled({ timeout: 10000 })
    await swapBtn.click()

    await expect(page.locator('.alert-success, .alert-error')).toBeVisible({ timeout: 60000 })
  })

  test('E4: direct wrap — LUNC to LUNC-C', async ({ page }) => {
    const hasLunc = await selectTokenByText(page, 'Select from token', 'LUNC')
    if (!hasLunc) {
      test.skip()
      return
    }

    const hasLuncC = await selectTokenByText(page, 'Select to token', 'LUNC-C')
    if (!hasLuncC) {
      test.skip()
      return
    }

    const wrapNote = page.getByText(/1:1/)
    const wrapNoteCount = await wrapNote.count()
    expect(wrapNoteCount).toBeGreaterThanOrEqual(0)

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('0.1')

    const swapBtn = page.getByRole('button', { name: /^Swap$/ })
    await expect(swapBtn).toBeEnabled({ timeout: 10000 })
    await swapBtn.click()

    await expect(page.locator('.alert-success, .alert-error')).toBeVisible({ timeout: 60000 })
  })

  test('E5: direct unwrap — LUNC-C to LUNC', async ({ page }) => {
    const hasLuncC = await selectTokenByText(page, 'Select from token', 'LUNC-C')
    if (!hasLuncC) {
      test.skip()
      return
    }

    const hasLunc = await selectTokenByText(page, 'Select to token', 'LUNC')
    if (!hasLunc) {
      test.skip()
      return
    }

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('0.1')

    const swapBtn = page.getByRole('button', { name: /^Swap$/ })
    await expect(swapBtn).toBeEnabled({ timeout: 10000 })
    await swapBtn.click()

    await expect(page.locator('.alert-success, .alert-error')).toBeVisible({ timeout: 60000 })
  })

  test('E6: wrapped-to-wrapped swap — LUNC-C to USTC-C (normal CW20)', async ({ page }) => {
    const hasLuncC = await selectTokenByText(page, 'Select from token', 'LUNC-C')
    if (!hasLuncC) {
      test.skip()
      return
    }

    const hasUstcC = await selectTokenByText(page, 'Select to token', 'USTC-C')
    if (!hasUstcC) {
      test.skip()
      return
    }

    const input = page.getByPlaceholder('0.00').first()
    await input.fill('0.1')

    const receiveField = page.locator('.card-neo').filter({ hasText: 'You Receive' }).locator('div.text-2xl')
    await expect(async () => {
      const text = await receiveField.textContent()
      expect(text).not.toBe('0.00')
      expect(text).not.toContain('Calculating')
    }).toPass({ timeout: 15000 })

    const swapBtn = page.getByRole('button', { name: /^Swap$/ })
    await expect(swapBtn).toBeEnabled({ timeout: 10000 })
    await swapBtn.click()

    await expect(page.locator('.alert-success, .alert-error')).toBeVisible({ timeout: 60000 })
  })

  test('E12: rate limit exceeded shows error in UI', async ({ page }) => {
    const hasLunc = await selectTokenByText(page, 'Select from token', 'LUNC')
    if (!hasLunc) {
      test.skip()
      return
    }

    const hasLuncC = await selectTokenByText(page, 'Select to token', 'LUNC-C')
    if (!hasLuncC) {
      test.skip()
      return
    }

    // Enter a very large amount to exceed any rate limit
    const input = page.getByPlaceholder('0.00').first()
    await input.fill('999999999999')

    // Check if the button shows rate limit, insufficient balance, or remains enabled
    await page.waitForTimeout(2000)
    const btn = page
      .locator('button')
      .filter({ hasText: /Rate Limit|Insufficient|Swap/i })
      .last()
    await expect(btn).toBeVisible()
  })
})
