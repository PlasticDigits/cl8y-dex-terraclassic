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
  await trigger.click()
  const list = page.getByRole('listbox', { name: ariaLabel })
  await expect(list).toBeVisible()

  // Type a short filter when the include token looks like a symbol (not an address).
  const filterHint = mustInclude.trim()
  if (filterHint.length >= 2 && !filterHint.toLowerCase().startsWith('terra1')) {
    await trigger.fill('')
    await trigger.type(filterHint.slice(0, Math.min(filterHint.length, 8)), { delay: 20 })
    await page.waitForTimeout(350)
  }

  const opts = list.getByRole('option')
  await expect(async () => {
    expect(await opts.count()).toBeGreaterThan(0)
  }).toPass({ timeout: 10_000 })

  const n = await opts.count()
  for (let i = 0; i < n; i++) {
    const txt = (await opts.nth(i).innerText()).replace(/\s+/g, ' ')
    if (!txt.includes(mustInclude)) continue
    if (mustNotInclude && txt.includes(mustNotInclude)) continue
    await opts.nth(i).click()
    return true
  }
  await page.keyboard.press('Escape')
  return false
}
