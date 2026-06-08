import { test, expect, type Locator, type Page } from '@playwright/test'

import { isChainOptional } from './chain'
import { selectTokenInCombobox } from './token-select'

const NATIVE_TOKEN_MSG =
  'Native LUNC/USTC not in token list; run bash scripts/deploy-dex-local.sh with LocalTerra (GitLab #201).'

const CW20_TOKEN_MSG =
  'No non-native CW20 in pay token list; deploy factory pairs before wrap tx E2E (GitLab #201).'

const NATIVE_WRAP_POOL_MSG =
  'No pool card with native auto-wrap; deploy native pairs for wrap-pool tx E2E (GitLab #201).'

const RECEIVE_WRAPPED_POOL_MSG =
  'No pool card with Receive as wrapped; deploy applicable pairs for wrap-pool withdraw E2E (GitLab #201).'

/** Select combobox token or skip (optional) / fail (strict). */
export async function requireTokenInCombobox(
  page: Page,
  ariaLabel: string,
  mustInclude: string,
  mustNotInclude?: string,
  detail = NATIVE_TOKEN_MSG
): Promise<void> {
  const ok = await selectTokenInCombobox(page, ariaLabel, mustInclude, mustNotInclude)
  if (ok) return
  if (isChainOptional()) {
    test.skip(true, detail)
  }
  expect(ok, detail).toBe(true)
}

/** Pick first pay-token option that is not LUNC or USTC. */
export async function requireNonNativeCw20PayOption(page: Page, detail = CW20_TOKEN_MSG): Promise<void> {
  const payList = page.getByRole('listbox', { name: 'Select token you pay' })
  await expect(payList).toBeVisible()
  const payOpts = payList.getByRole('option')
  const pn = await payOpts.count()
  for (let i = 0; i < pn; i++) {
    const t = (await payOpts.nth(i).innerText()).replace(/\s+/g, ' ')
    if (!t.includes('LUNC') && !t.includes('USTC')) {
      await payOpts.nth(i).click()
      return
    }
  }
  await page.keyboard.press('Escape')
  if (isChainOptional()) {
    test.skip(true, detail)
  }
  expect(false, detail).toBe(true)
}

/** Pick first receive option that is not LUNC or USTC. */
export async function requireNonNativeCw20ReceiveOption(page: Page, detail = CW20_TOKEN_MSG): Promise<void> {
  const recvList = page.getByRole('listbox', { name: 'Select token you receive' })
  await expect(recvList).toBeVisible()
  const recvOpts = recvList.getByRole('option')
  const n = await recvOpts.count()
  for (let i = 0; i < n; i++) {
    const t = (await recvOpts.nth(i).innerText()).replace(/\s+/g, ' ')
    if (!t.includes('LUNC') && !t.includes('USTC')) {
      await recvOpts.nth(i).click()
      return
    }
  }
  await page.keyboard.press('Escape')
  if (isChainOptional()) {
    test.skip(true, detail)
  }
  expect(false, detail).toBe(true)
}

export async function requirePoolCardWithNativeWrap(
  scope: Page | Locator,
  detail = NATIVE_WRAP_POOL_MSG
): Promise<void> {
  const nativeCheckbox = scope.getByText(/auto-wrap/i)
  if ((await nativeCheckbox.count()) > 0) return
  if (isChainOptional()) {
    test.skip(true, detail)
  }
  expect(await nativeCheckbox.count(), detail).toBeGreaterThan(0)
}

export async function requirePoolCardWithReceiveWrapped(
  scope: Page | Locator,
  detail = RECEIVE_WRAPPED_POOL_MSG
): Promise<void> {
  const receiveWrappedCheckbox = scope.getByText(/Receive as wrapped/i)
  if ((await receiveWrappedCheckbox.count()) > 0) return
  if (isChainOptional()) {
    test.skip(true, detail)
  }
  expect(await receiveWrappedCheckbox.count(), detail).toBeGreaterThan(0)
}
