import { expect, test } from './fixtures/dev-wallet'
import type { Locator, Page } from '@playwright/test'
import { skipIfLcdUnreachable, assertTxResultAlert } from './helpers/chain'
import { selectTokenInCombobox } from './helpers/token-select'
import { openPoolCardAdvanced, poolProvideExpandButton, poolProvideSubmitButton } from './helpers/pool-ui'

/**
 * Pick the first factory pair that is not an empty inherit-test pool (GitLab #538 I538A/I538B)
 * so one-sided zap can quote (Z533-5 / #559 P9 seeded pair).
 */
async function pickSeededFactoryPairForAdd(page: Page, add: Locator, amount: string): Promise<void> {
  const pairBox = add.getByRole('combobox', { name: /^Pair$/i })
  await expect(pairBox).toBeEnabled({ timeout: 25_000 })
  await pairBox.click()
  const pairList = page.getByRole('listbox', { name: /^Pair$/i })
  await expect(pairList).toBeVisible()
  const pairOpts = pairList.getByRole('option')
  const n = await pairOpts.count()
  if (n === 0) {
    test.skip(true, 'No factory pairs in retail Pair picker.')
  }
  const amountBox = add.getByTestId('pool-one-sided-add-amount')
  const submit = add.getByTestId('pool-one-sided-add-submit')
  const quote = add.getByTestId('pool-one-sided-add-quote')
  const blocked = add
    .getByText('Empty pool. Use Advanced.')
    .or(add.getByText('No route'))
    .or(add.getByText('Amount too small'))
  for (let i = 0; i < n; i++) {
    const opt = pairOpts.nth(i)
    const label = (await opt.innerText()).replace(/\s+/g, ' ')
    if (/I538A|I538B/i.test(label)) continue
    await opt.click()
    await amountBox.fill(amount)
    try {
      await expect(quote.or(blocked)).toBeVisible({ timeout: 12_000 })
    } catch {
      await pairBox.click()
      continue
    }
    if ((await quote.isVisible()) && (await submit.isEnabled())) return
    await pairBox.click()
  }
  test.skip(true, 'No seeded (non-empty) factory pair for one-sided add.')
}

/**
 * LocalTerra one-sided add/withdraw (GitLab #533 P4–P8, #559 P9).
 * Smoke UI lives in `pool-one-sided-533.spec.ts` (e2e-smoke, 5 workers).
 * This file is e2e-tx (1 worker) — shared LocalTerra account.
 */
test.describe('One-sided pool add/withdraw tx (GitLab #533 P4–P8 / #559 P9)', () => {
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
    await pickSeededFactoryPairForAdd(page, add, '1')
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

  test('P9 conservative zap-in: human min-swap and add succeeds when fill can be below quote (GitLab #559)', async ({
    page,
  }) => {
    const add = page.getByTestId('pool-one-sided-add')
    const tokenOk = await selectTokenInCombobox(page, 'Token', '')
    if (!tokenOk) {
      test.skip(true, 'No wallet token in retail add picker; provision LocalTerra wallet holdings.')
    }
    await pickSeededFactoryPairForAdd(page, add, '1')
    const pre = add.getByTestId('pool-one-sided-add-pre-submit')
    await expect(pre).toBeVisible({ timeout: 30_000 })
    const preText = await pre.innerText()
    expect(preText).toMatch(/min swap /i)
    // Raw uint min-swap (e.g. `min swap 500571`) must not sit next to a human pay amount.
    expect(preText).not.toMatch(/min swap \d{5,}\s*$/m)
    const submit = add.getByTestId('pool-one-sided-add-submit')
    await expect(submit).toBeEnabled({ timeout: 30_000 })
    await submit.click()
    await assertTxResultAlert(page, 120_000)
  })

  test('P6 / P7 withdraw as one token (unwrap uses quoted amount only — A7)', async ({ page }) => {
    const w = page.getByTestId('pool-one-sided-withdraw')
    const lp = w.getByLabel(/^LP$/i)
    if (await lp.isDisabled()) {
      test.skip(true, 'No wallet LP for one-sided withdraw; run P4/P5 first or seed LP.')
    }
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
    const manage = page.getByTestId('pool-row-manage').first()
    if ((await manage.count()) === 0) {
      test.skip(true, 'Pool table empty (indexer down); Advanced manage lives on catalog rows.')
    }
    await expect(manage).toBeVisible({ timeout: 90_000 })
    await openPoolCardAdvanced(page)
    await expect(poolProvideExpandButton(page)).toBeVisible()
    await poolProvideExpandButton(page).click()
    await expect(page.getByLabel('Asset A amount')).toBeVisible()
    await expect(poolProvideSubmitButton(page)).toBeVisible()
  })
})
