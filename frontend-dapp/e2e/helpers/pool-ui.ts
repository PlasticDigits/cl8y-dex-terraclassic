import type { Locator } from '@playwright/test'

/**
 * Pool cards expose a toggle and a submit button with the same accessible name
 * once the provide/withdraw form is expanded. Use `.first()` to expand, `.last()` to submit.
 *
 * Invariant (GitLab #201): tx specs must not use ambiguous `getByRole('Provide Liquidity')`
 * on an expanded card — Playwright strict mode resolves both controls.
 */
export function poolProvideExpandButton(pairCard: Locator): Locator {
  return pairCard.getByRole('button', { name: /^Provide Liquidity$/i }).first()
}

export function poolProvideSubmitButton(pairCard: Locator): Locator {
  return pairCard.getByRole('button', { name: /^Provide Liquidity$/i }).last()
}

export function poolWithdrawExpandButton(pairCard: Locator): Locator {
  return pairCard.getByRole('button', { name: /^Withdraw Liquidity$/i }).first()
}

export function poolWithdrawSubmitButton(pairCard: Locator): Locator {
  return pairCard.getByRole('button', { name: /^Withdraw Liquidity$/i }).last()
}

export function poolCardAdvanced(scope: Locator) {
  return scope.getByTestId('pool-card-advanced').first()
}

export async function openPoolCardAdvanced(scope: Locator): Promise<void> {
  const details = poolCardAdvanced(scope)
  await details.waitFor({ state: 'visible' })
  if ((await details.getAttribute('open')) == null) {
    await details.locator('summary').click()
  }
}
