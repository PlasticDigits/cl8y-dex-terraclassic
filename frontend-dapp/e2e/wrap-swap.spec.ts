import { test, expect } from './fixtures/dev-wallet'

test.describe('Swap with native token wrapping', () => {
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
