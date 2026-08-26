import type { Locator, Page } from '@playwright/test'

/**
 * Tab controls use `Select …` aria-labels so they do not collide with submit
 * CTAs that reuse the visible action name (GitLab #201 / #660).
 */
export type PoolManageAction = 'provide' | 'withdraw' | 'zap-add' | 'zap-withdraw'

export function poolManageTab(scope: Locator | Page, action: PoolManageAction): Locator {
  return scope.getByTestId(`pool-manage-tab-${action}`)
}

export function poolProvideExpandButton(scope: Locator | Page): Locator {
  return poolManageTab(scope, 'provide')
}

export function poolProvideSubmitButton(scope: Locator | Page): Locator {
  return scope.getByRole('button', { name: /^Provide Liquidity$/i })
}

export function poolWithdrawExpandButton(scope: Locator | Page): Locator {
  return poolManageTab(scope, 'withdraw')
}

export function poolWithdrawSubmitButton(scope: Locator | Page): Locator {
  return scope.getByRole('button', { name: /^Withdraw Liquidity$/i })
}

export function poolReceiveWrappedCheckbox(scope: Locator | Page): Locator {
  return scope.getByRole('checkbox', { name: /Receive as wrapped/i })
}

export function firstFactoryPairGroup(scope: Locator | Page): Locator {
  return scope
    .locator('[data-testid="pool-pair-group"]')
    .filter({ has: scope.locator('[data-testid="pool-row-factory"]') })
    .first()
}

export async function openPoolManage(scope: Locator | Page, action?: PoolManageAction): Promise<void> {
  const manage = scope.getByTestId('pool-row-manage').first()
  if ((await manage.count()) > 0) {
    const expanded = await manage.getAttribute('aria-expanded')
    if (expanded !== 'true') {
      await manage.click()
    }
  }
  await scope.getByTestId('pool-manage-actions').waitFor({ state: 'visible' })
  if (action) {
    const tab = poolManageTab(scope, action)
    if ((await tab.getAttribute('aria-pressed')) !== 'true') {
      await tab.click()
    }
  }
}

/** Opens Manage on the first visible row. No Advanced disclosure (#660). */
export async function openPoolCardAdvanced(scope: Locator | Page): Promise<void> {
  await openPoolManage(scope)
}

export async function openFirstFactoryManage(page: Page, action?: PoolManageAction): Promise<Locator> {
  const group = firstFactoryPairGroup(page)
  await group.waitFor({ state: 'visible', timeout: 90_000 })
  await openPoolManage(group, action)
  return group
}
