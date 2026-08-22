import type { Page } from '@playwright/test'
import { expect } from '@playwright/test'

export const ARIA_SELECT_TOKEN_PAY = 'Select token you pay'
export const ARIA_SELECT_TOKEN_RECEIVE = 'Select token you receive'

/** Swap pay/receive use TokenSearchSelect combobox (GitLab #481); Mint still uses TokenSelect button. */
export function payTokenTrigger(page: Page) {
  return page.getByRole('combobox', { name: ARIA_SELECT_TOKEN_PAY })
}

export function receiveTokenTrigger(page: Page) {
  return page.getByRole('combobox', { name: ARIA_SELECT_TOKEN_RECEIVE })
}

/** Pay/receive triggers stay disabled until factory tokens load. */
export async function waitForPayTokenTriggerEnabled(page: Page, timeout = 25_000) {
  await expect(payTokenTrigger(page)).toBeEnabled({ timeout })
}

/** Opens the pay combobox, asserts at least one option, closes with Escape. */
export async function expectPayTokenListPopulated(page: Page, timeout = 20_000) {
  await waitForPayTokenTriggerEnabled(page, timeout)
  await payTokenTrigger(page).click()
  const list = page.getByRole('listbox', { name: ARIA_SELECT_TOKEN_PAY })
  await expect(list).toBeVisible()
  await expect(async () => {
    expect(await list.getByRole('option').count()).toBeGreaterThan(0)
  }).toPass({ timeout })
  await page.keyboard.press('Escape')
}

/** Swap needs at least two distinct tokens in the pay list. */
export async function expectAtLeastTwoPayTokenOptions(page: Page, timeout = 25_000) {
  await waitForPayTokenTriggerEnabled(page, timeout)
  await payTokenTrigger(page).click()
  const list = page.getByRole('listbox', { name: ARIA_SELECT_TOKEN_PAY })
  await expect(list).toBeVisible()
  await expect(async () => {
    expect(await list.getByRole('option').count()).toBeGreaterThan(1)
  }).toPass({ timeout: 20_000 })
  await page.keyboard.press('Escape')
}

/**
 * Opens token combobox by aria-label, optionally types to filter, picks first matching option.
 * Returns whether a match was clicked.
 */
export async function selectTokenInCombobox(
  page: Page,
  ariaLabel: string,
  mustInclude: string,
  mustNotInclude?: string
): Promise<boolean> {
  const trigger = page.getByRole('combobox', { name: ariaLabel })
  await expect(trigger).toBeEnabled({ timeout: 25_000 })
  // Escape after a prior open can leave the combobox focused; click then would not fire onFocus.
  await page.keyboard.press('Escape')
  await trigger.blur()
  await trigger.click()
  const list = page.getByRole('listbox', { name: ariaLabel })
  if (!(await list.isVisible().catch(() => false))) {
    await trigger.click()
  }
  await expect(list).toBeVisible({ timeout: 10_000 })

  const opts = list.getByRole('option')
  await expect(async () => {
    expect(await opts.count()).toBeGreaterThan(0)
  }).toPass({ timeout: 10_000 })

  const tryPick = async (): Promise<boolean> => {
    const n = await opts.count()
    for (let i = 0; i < n; i++) {
      const txt = (await opts.nth(i).innerText()).replace(/\s+/g, ' ')
      if (!txt.includes(mustInclude)) continue
      if (mustNotInclude && txt.includes(mustNotInclude)) continue
      await opts.nth(i).click()
      return true
    }
    return false
  }

  // Filter only after the unfiltered list is populated. Empty filter (USTR missing)
  // must return false so callers can try JADE/RUBY — do not hang on 0 options.
  const filterHint = mustInclude.trim()
  if (filterHint.length >= 2 && !filterHint.toLowerCase().startsWith('terra1')) {
    await trigger.fill('')
    await trigger.type(filterHint.slice(0, Math.min(filterHint.length, 8)), { delay: 20 })
    await page.waitForTimeout(400)
    if (await tryPick()) return true
    await trigger.fill('')
    await page.waitForTimeout(400)
    await expect(async () => {
      expect(await opts.count()).toBeGreaterThan(0)
    }).toPass({ timeout: 10_000 })
  }

  if (await tryPick()) return true
  await page.keyboard.press('Escape')
  return false
}
