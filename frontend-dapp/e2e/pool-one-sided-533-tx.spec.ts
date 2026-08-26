import { expect, test } from './fixtures/dev-wallet'
import type { Locator, Page } from '@playwright/test'
import { skipIfLcdUnreachable, assertTxResultAlert } from './helpers/chain'
import { selectTokenInCombobox } from './helpers/token-select'
import { openFirstFactoryManage, poolProvideExpandButton, poolProvideSubmitButton } from './helpers/pool-ui'

function factoryPairGroups(page: Page): Locator {
  return page.locator('[data-testid="pool-pair-group"]').filter({
    has: page.locator('[data-testid="pool-row-factory"]'),
  })
}

async function collapseManage(group: Locator): Promise<void> {
  const manage = group.getByTestId('pool-row-manage')
  if ((await manage.getAttribute('aria-expanded')) === 'true') {
    await manage.click()
  }
}

/** Open Zap Add on the first non-inherit-test factory pair (#660 pair-scoped Manage). */
async function openZapAddOnFactoryPair(page: Page): Promise<Locator> {
  await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
  const groups = factoryPairGroups(page)
  const n = await groups.count()
  if (n === 0) {
    test.skip(true, 'No factory pairs in the pool table.')
  }
  for (let i = 0; i < n; i++) {
    const group = groups.nth(i)
    const label = (await group.getByTestId('pool-pair-row').innerText()).replace(/\s+/g, ' ')
    if (/I538A|I538B/i.test(label)) continue
    const manage = group.getByTestId('pool-row-manage')
    if ((await manage.getAttribute('aria-expanded')) !== 'true') {
      await manage.click()
    }
    const zapTab = group.getByTestId('pool-manage-tab-zap-add')
    if ((await zapTab.count()) === 0) {
      await collapseManage(group)
      continue
    }
    await zapTab.click()
    const add = group.getByTestId('pool-one-sided-add')
    await expect(add).toBeVisible()
    return add
  }
  test.skip(true, 'No factory pair with Zap Add.')
  return page.getByTestId('pool-one-sided-add')
}

/**
 * LocalTerra one-sided add/withdraw (GitLab #533 P4–P8, #559 P9, #660).
 * Smoke UI lives in `pool-one-sided-533.spec.ts` (e2e-smoke, 5 workers).
 * This file is e2e-tx (1 worker) — shared LocalTerra account.
 */
test.describe('One-sided pool add/withdraw tx (GitLab #533 P4–P8 / #559 P9 / #660)', () => {
  test.describe.configure({ mode: 'serial' })

  test.beforeEach(async ({ page, connectWallet, request }) => {
    await skipIfLcdUnreachable(request)
    await connectWallet
    await page.getByRole('link', { name: 'Pool' }).click()
    await page.waitForURL(/\/pool/)
    await expect(page.getByTestId('pool-pairs-table')).toBeVisible({ timeout: 90_000 })
    await expect(page.getByTestId('pool-one-sided-add')).toHaveCount(0)
  })

  test('P4 one-sided add with a pair CW20', async ({ page }) => {
    const add = await openZapAddOnFactoryPair(page)
    const tokenOk = await selectTokenInCombobox(page, 'Token', '')
    if (!tokenOk) {
      test.skip(true, 'No wallet token in retail add picker; provision LocalTerra wallet holdings.')
    }
    await add.getByTestId('pool-one-sided-add-amount').fill('1')
    const submit = add.getByTestId('pool-one-sided-add-submit')
    await expect(submit).toBeEnabled({ timeout: 30_000 })
    await submit.click()
    await assertTxResultAlert(page, 120_000)
  })

  test('P5 one-sided add with native LUNC (wrap + zap)', async ({ page }) => {
    const groups = factoryPairGroups(page)
    const n = await groups.count()
    let add: Locator | null = null
    for (let i = 0; i < n; i++) {
      const group = groups.nth(i)
      const txt = (await group.getByTestId('pool-pair-row').innerText()).replace(/\s+/g, ' ')
      if (!/cLUNC/i.test(txt) && i < n - 1) continue
      const manage = group.getByTestId('pool-row-manage')
      if ((await manage.getAttribute('aria-expanded')) !== 'true') {
        await manage.click()
      }
      const zapTab = group.getByTestId('pool-manage-tab-zap-add')
      if ((await zapTab.count()) === 0) {
        await collapseManage(group)
        continue
      }
      await zapTab.click()
      add = group.getByTestId('pool-one-sided-add')
      break
    }
    if (!add) {
      test.skip(true, 'No factory pair for native zap-in.')
    }
    const luncOk = await selectTokenInCombobox(page, 'Token', 'LUNC', 'cLUNC')
    if (!luncOk) {
      test.skip(true, 'Native LUNC not in retail add picker; wrap env + wallet uluna required.')
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
    const add = await openZapAddOnFactoryPair(page)
    const tokenOk = await selectTokenInCombobox(page, 'Token', '')
    if (!tokenOk) {
      test.skip(true, 'No wallet token in retail add picker; provision LocalTerra wallet holdings.')
    }
    await add.getByTestId('pool-one-sided-add-amount').fill('1')
    const pre = add.getByTestId('pool-one-sided-add-pre-submit')
    await expect(pre).toBeVisible({ timeout: 30_000 })
    const preText = await pre.innerText()
    expect(preText).toMatch(/Zap Add/i)
    expect(preText).toMatch(/min swap /i)
    expect(preText).not.toMatch(/min swap \d{5,}\s*$/m)
    const submit = add.getByTestId('pool-one-sided-add-submit')
    await expect(submit).toBeEnabled({ timeout: 30_000 })
    await submit.click()
    await assertTxResultAlert(page, 120_000)
  })

  test('P6 / P7 withdraw as one token (unwrap uses quoted amount only — A7)', async ({ page }) => {
    const groups = factoryPairGroups(page)
    const n = await groups.count()
    let w: Locator | null = null
    for (let i = 0; i < n; i++) {
      const group = groups.nth(i)
      const manage = group.getByTestId('pool-row-manage')
      if ((await manage.getAttribute('aria-expanded')) !== 'true') {
        await manage.click()
      }
      const tab = group.getByTestId('pool-manage-tab-zap-withdraw')
      if ((await tab.count()) === 0) {
        await collapseManage(group)
        continue
      }
      await tab.click()
      const card = group.getByTestId('pool-one-sided-withdraw')
      const empty = card.getByTestId('pool-one-sided-withdraw-empty-lp')
      if (await empty.isVisible().catch(() => false)) {
        await collapseManage(group)
        continue
      }
      w = card
      break
    }
    if (!w) {
      test.skip(true, 'No wallet LP for one-sided withdraw; run P4/P5 first or seed LP.')
    }
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

  test('P8 empty pool: zap disabled; Provide Liquidity still reachable', async ({ page }) => {
    await openFirstFactoryManage(page, 'provide')
    await expect(poolProvideExpandButton(page)).toBeVisible()
    await expect(page.getByLabel('Asset A amount')).toBeVisible()
    await expect(poolProvideSubmitButton(page)).toBeVisible()
    await expect(page.getByTestId('pool-card-advanced')).toHaveCount(0)
  })
})
