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

/** "Receive as wrapped" toggle on withdraw forms for native-capable pairs. */
export function poolReceiveWrappedCheckbox(pairCard: Locator): Locator {
  return pairCard.getByRole('checkbox', { name: /Receive as wrapped/i })
}
