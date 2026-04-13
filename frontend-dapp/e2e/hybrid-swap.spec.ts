import { test, expect } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable } from './helpers/chain'

test.describe('Hybrid swap UI (LocalTerra)', () => {
  test('shows hybrid book disclosure and documentation link when book leg is set', async ({
    page,
    connectWallet,
    request,
  }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet
    await page.waitForLoadState('networkidle')

    const fromSelect = page.getByLabel('Select from token')
    await expect(async () => {
      const n = await fromSelect.locator('option').count()
      expect(n).toBeGreaterThan(1)
    }).toPass({ timeout: 25_000 })

    const firstCw = await fromSelect.locator('option').evaluateAll((opts) => {
      for (const o of opts) {
        const v = (o as HTMLOptionElement).value
        if (v.startsWith('terra1')) return v
      }
      return ''
    })
    if (!firstCw) {
      test.skip(true, 'No CW20 option in from-token list for hybrid controls.')
    }

    await fromSelect.selectOption(firstCw)
    const toSelect = page.getByLabel('Select to token')
    const toVal = await toSelect.locator('option').evaluateAll((opts, from) => {
      for (const o of opts) {
        const v = (o as HTMLOptionElement).value
        if (v && v !== from && v.startsWith('terra1')) return v
      }
      return ''
    }, firstCw)
    if (!toVal) {
      test.skip(true, 'No second CW20 token for pair selection.')
    }
    await toSelect.selectOption(toVal)

    await page.getByRole('button', { name: 'Settings' }).click()

    const hybridHeading = page.getByText('Direct swap: limit book leg')
    if ((await hybridHeading.count()) === 0) {
      test.skip(true, 'No direct CW20 route for selected pair; hybrid controls hidden.')
    }

    await page.getByRole('checkbox', { name: /Route part of input through the limit book/i }).check()
    await page.locator('.card-neo').filter({ hasText: 'Book leg amount' }).getByPlaceholder('0.0').fill('0.01')
    await page.getByPlaceholder('0.00').first().fill('1')

    const alert = page.getByRole('alert')
    await expect(alert).toBeVisible({ timeout: 15_000 })
    await expect(alert).toContainText(/limit book/i)
    const doc = alert.getByRole('link', { name: /docs\/limit-orders\.md/i })
    await expect(doc).toHaveAttribute('href', /limit-orders\.md/)
  })
})
