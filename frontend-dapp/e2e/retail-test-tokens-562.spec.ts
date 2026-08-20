import { test, expect } from './fixtures/dev-wallet'
import { readFrontendEnvLocal } from './helpers/create-pair-picker'
import { ARIA_SELECT_TOKEN_PAY, payTokenTrigger, waitForPayTokenTriggerEnabled } from './helpers/token-select'

/**
 * GitLab #562 P1 / #573 M573-4: LocalTerra (`VITE_NETWORK=local`) must still list
 * soft-launch gems in Swap pay. Production hide is mainnet-only.
 */
test.describe('LocalTerra retail gems still listed (GitLab #562 P1)', () => {
  test('pay picker lists every factory token including gems', async ({ page }) => {
    const env = readFrontendEnvLocal()
    const ember = env.VITE_TOKEN_EMBER_ADDRESS?.trim()
    test.skip(!ember, 'VITE_TOKEN_EMBER_ADDRESS missing — need make deploy-local / .env.local')

    await page.goto('/')
    await waitForPayTokenTriggerEnabled(page)

    const trigger = payTokenTrigger(page)
    await trigger.click()
    const list = page.getByRole('listbox', { name: ARIA_SELECT_TOKEN_PAY })
    await expect(list).toBeVisible()

    const byAddr = list.getByTestId(`token-option-${ember}`)
    if ((await byAddr.count()) === 0) {
      await trigger.fill('')
      await trigger.pressSequentially('EMBER', { delay: 20 })
    }
    await expect(list.getByTestId(`token-option-${ember}`)).toBeVisible({ timeout: 15_000 })
    await expect(list.getByRole('option').filter({ hasText: /EMBER/i }).first()).toBeVisible()
    await page.keyboard.press('Escape')
  })
})
