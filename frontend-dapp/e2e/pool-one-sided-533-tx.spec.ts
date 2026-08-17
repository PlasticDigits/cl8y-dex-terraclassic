import { expect, test } from './fixtures/dev-wallet'
import { skipIfLcdUnreachable, assertTxResultAlert } from './helpers/chain'
import { selectTokenInCombobox } from './helpers/token-select'
import { openPoolCardAdvanced, poolProvideExpandButton, poolProvideSubmitButton } from './helpers/pool-ui'

/**
 * LocalTerra one-sided add/withdraw (GitLab #533 P4–P8).
 * Smoke UI lives in `pool-one-sided-533.spec.ts` (e2e-smoke, 5 workers).
 * This file is e2e-tx (1 worker) — shared LocalTerra account.
 */
test.describe('One-sided pool add/withdraw tx (GitLab #533 P4–P8)', () => {
  test.describe.configure({ mode: 'serial' })

  test.beforeEach(async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet
    await page.getByRole('link', { name: 'Pool' }).click()
    await page.waitForURL(/\/pool/)
    await expect(page.getByTestId('pool-one-sided-add')).toBeVisible({ timeout: 90_000 })
  })

  test('P4 one-sided add with a pair CW20', async ({ page }) => {
    const add = page.getByTestId('pool-one-sided-add')
    const tokenOk = await selectTokenInCombobox(page, 'Token', '')
    if (!tokenOk) {
      test.skip(true, 'No wallet token in retail add picker; provision LocalTerra wallet holdings.')
    }
    const pairBox = add.getByRole('combobox', { name: /^Pair$/i })
    await expect(pairBox).toBeEnabled({ timeout: 25_000 })
    await pairBox.click()
    const pairList = page.getByRole('listbox', { name: /^Pair$/i })
    await expect(pairList).toBeVisible()
    const pairOpts = pairList.getByRole('option')
    if ((await pairOpts.count()) === 0) {
      test.skip(true, 'No factory pairs in retail Pair picker.')
    }
    await pairOpts.first().click()
    await add.getByTestId('pool-one-sided-add-amount').fill('1')
    const submit = add.getByTestId('pool-one-sided-add-submit')
    await expect(submit).toBeEnabled({ timeout: 30_000 })
    await submit.click()
    await assertTxResultAlert(page, 120_000)
  })

  test('P5 one-sided add with native LUNC (wrap + zap)', async ({ page }) => {
    const add = page.getByTestId('pool-one-sided-add')
    // "cLUNC" includes "LUNC" — exclude wrapped so wrapDenom is set (GitLab #539).
    const luncOk = await selectTokenInCombobox(page, 'Token', 'LUNC', 'cLUNC')
    if (!luncOk) {
      test.skip(true, 'Native LUNC not in retail add picker; wrap env + wallet uluna required.')
    }
    const pairBox = add.getByRole('combobox', { name: /^Pair$/i })
    await pairBox.click()
    const pairList = page.getByRole('listbox', { name: /^Pair$/i })
    const pairOpts = pairList.getByRole('option')
    if ((await pairOpts.count()) === 0) {
      test.skip(true, 'No factory pairs for native zap-in.')
    }
    const pairCount = await pairOpts.count()
    let pickedWrapPair = false
    for (let i = 0; i < pairCount; i++) {
      const txt = (await pairOpts.nth(i).innerText()).replace(/\s+/g, ' ')
      if (/cLUNC/i.test(txt)) {
        await pairOpts.nth(i).click()
        pickedWrapPair = true
        break
      }
    }
    if (!pickedWrapPair) {
      await pairOpts.first().click()
    }
    await add.getByTestId('pool-one-sided-add-amount').fill('10')
    const quote = add.getByTestId('pool-one-sided-add-quote')
    await expect(quote).toBeVisible({ timeout: 30_000 })
    await expect(quote).toContainText(/Wrap/i)
    const submit = add.getByTestId('pool-one-sided-add-submit')
    await expect(submit).toBeEnabled({ timeout: 30_000 })
    await submit.click()
    await assertTxResultAlert(page, 120_000)
  })

  test('P6 / P7 withdraw as one token (unwrap uses quoted amount only — A7)', async ({ page }) => {
    const w = page.getByTestId('pool-one-sided-withdraw')
    const lp = w.getByLabel(/^LP$/i)
    await lp.click()
    const lpOpts = page.getByRole('option')
    if ((await lpOpts.count()) === 0) {
      test.skip(true, 'No wallet LP for one-sided withdraw; run P4/P5 first or seed LP.')
    }
    await lpOpts.first().click()
    const asBox = w.getByRole('combobox', { name: /Withdraw as/i })
    await asBox.click()
    const asOpts = page.getByRole('listbox', { name: /Withdraw as/i }).getByRole('option')
    if ((await asOpts.count()) === 0) {
      test.skip(true, 'No withdraw-as tokens for selected LP.')
    }
    await asOpts.first().click()
    await w.getByTestId('pool-one-sided-withdraw-max').click()
    const submit = w.getByTestId('pool-one-sided-withdraw-submit')
    await expect(submit).toBeEnabled({ timeout: 30_000 })
    await submit.click()
    await assertTxResultAlert(page, 120_000)
  })

  test('P8 empty pool: one-sided disabled; Advanced two-sided still reachable', async ({ page }) => {
    await expect(page.getByTestId('pool-one-sided-add-submit')).toBeVisible()
    const advanced = page.getByTestId('pool-card-advanced').first()
    await expect(advanced).toBeVisible({ timeout: 90_000 })
    await openPoolCardAdvanced(page)
    await expect(poolProvideExpandButton(page)).toBeVisible()
    await poolProvideExpandButton(page).click()
    await expect(page.getByLabel('Asset A amount')).toBeVisible()
    await expect(poolProvideSubmitButton(page)).toBeVisible()
  })
})
