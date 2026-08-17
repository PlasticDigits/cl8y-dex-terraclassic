import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import type { Page } from '@playwright/test'
import { expect } from '@playwright/test'

const here = path.dirname(fileURLToPath(import.meta.url))

export const ARIA_SELECT_TOKEN_A = 'Select token A'
export const ARIA_SELECT_TOKEN_B = 'Select token B'

export function readFrontendEnvLocal(): Record<string, string> {
  const envLocal = path.join(here, '..', '..', '.env.local')
  const out: Record<string, string> = {}
  if (!fs.existsSync(envLocal)) return out
  for (const line of fs.readFileSync(envLocal, 'utf8').split('\n')) {
    const m = line.match(/^(VITE_[A-Z0-9_]+)=(.*)$/)
    if (!m) continue
    out[m[1]] = m[2].trim().replace(/^["']|["']$/g, '')
  }
  return out
}

export function createPairTokenA(page: Page) {
  return page.getByRole('combobox', { name: ARIA_SELECT_TOKEN_A })
}

export function createPairTokenB(page: Page) {
  return page.getByRole('combobox', { name: ARIA_SELECT_TOKEN_B })
}

export async function openCreatePairCustom(page: Page, leg: 'a' | 'b') {
  const testId = leg === 'a' ? 'create-pair-custom-toggle-token-a' : 'create-pair-custom-toggle-token-b'
  await page.getByTestId(testId).click()
}

export async function fillCreatePairCustom(page: Page, leg: 'a' | 'b', address: string) {
  await openCreatePairCustom(page, leg)
  const label = leg === 'a' ? /Token A Contract Address/i : /Token B Contract Address/i
  await page.getByLabel(label).fill(address)
}

/** Click a listed option by contract address (`data-testid=token-option-…`). */
export async function pickCreatePairTokenByAddress(page: Page, ariaLabel: string, address: string): Promise<boolean> {
  const trigger = page.getByRole('combobox', { name: ariaLabel })
  await expect(trigger).toBeEnabled({ timeout: 25_000 })
  await trigger.click()
  const list = page.getByRole('listbox', { name: ariaLabel })
  await expect(list).toBeVisible()
  const opt = list.getByTestId(`token-option-${address}`)
  if ((await opt.count()) === 0) {
    await page.keyboard.press('Escape')
    return false
  }
  await opt.click()
  return true
}

/** Click a listed option by symbol. Does not type-filter (Create Pair list is small). */
export async function pickCreatePairToken(page: Page, ariaLabel: string, symbol: string): Promise<boolean> {
  const trigger = page.getByRole('combobox', { name: ariaLabel })
  await expect(trigger).toBeEnabled({ timeout: 25_000 })
  await trigger.click()
  const list = page.getByRole('listbox', { name: ariaLabel })
  await expect(list).toBeVisible()
  const opts = list.getByRole('option')
  await expect(async () => {
    expect(await opts.count()).toBeGreaterThan(0)
  }).toPass({ timeout: 10_000 })

  const n = await opts.count()
  const needle = symbol.trim().toLowerCase()
  for (let i = 0; i < n; i++) {
    const txt = (await opts.nth(i).innerText()).replace(/\s+/g, ' ').toLowerCase()
    if (!txt.includes(needle)) continue
    await opts.nth(i).click()
    return true
  }
  await page.keyboard.press('Escape')
  return false
}
