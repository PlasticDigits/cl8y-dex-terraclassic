import type { Page } from '@playwright/test'
import { expect } from '@playwright/test'

export const ARIA_SELECT_TOKEN_PAY = 'Select token you pay'
export const ARIA_SELECT_TOKEN_RECEIVE = 'Select token you receive'

export function payTokenTrigger(page: Page) {
  return page.getByRole('button', { name: ARIA_SELECT_TOKEN_PAY })
}

export function receiveTokenTrigger(page: Page) {
  return page.getByRole('button', { name: ARIA_SELECT_TOKEN_RECEIVE })
}

/** Pay/receive triggers stay disabled until the indexer returns tokens. */
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
 * Opens combobox by aria-label, picks first option matching substring rules, returns whether a match was clicked.
 */
export async function selectTokenInCombobox(
  page: Page,
  ariaLabel: string,
  mustInclude: string,
  mustNotInclude?: string
): Promise<boolean> {
  const trigger = page.getByRole('button', { name: ariaLabel })
  await expect(trigger).toBeEnabled({ timeout: 25_000 })
  await trigger.click()
  const list = page.getByRole('listbox', { name: ariaLabel })
  await expect(list).toBeVisible()
  const opts = list.getByRole('option')
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
