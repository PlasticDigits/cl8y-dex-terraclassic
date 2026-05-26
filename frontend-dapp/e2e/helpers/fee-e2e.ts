import { test, expect, type Page } from '@playwright/test'

import { isChainOptional } from './chain'

const REGISTER_CTA_MSG =
  'No self-service Register buttons; ensure fee-discount tiers 1–9 are deployed and dev wallet holds tier-1 CL8Y (scripts/e2e-provision-dev-wallet.sh, GitLab #201).'

/** Fee tier tx E2E requires at least one Register CTA after tiers load. */
export async function requireSelfServiceRegisterButtons(page: Page, detail = REGISTER_CTA_MSG): Promise<void> {
  const registerBtns = page.getByRole('button', { name: /^Register$/ })
  await expect(async () => {
    expect(await registerBtns.count()).toBeGreaterThan(0)
  })
    .toPass({ timeout: 45_000 })
    .catch(() => {})

  if ((await registerBtns.count()) > 0) return
  if (isChainOptional()) {
    test.skip(true, detail)
  }
  expect(await registerBtns.count(), detail).toBeGreaterThan(0)
}
