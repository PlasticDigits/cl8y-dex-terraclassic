import { expect, type Locator, type Page } from '@playwright/test'

import { assertSwapCtaNotBlocked } from './chain'

/** You Receive amount: editable input on direct pairs (#713), otherwise a quote display. */
export function swapYouReceiveAmountDisplay(page: Page) {
  return page.getByTestId('swap-you-receive')
}

/** Human amount currently shown in You Receive (input value or display text). */
export async function readSwapYouReceiveAmount(page: Page): Promise<string> {
  const el = swapYouReceiveAmountDisplay(page)
  const tag = await el.evaluate((n) => n.tagName.toLowerCase())
  if (tag === 'input' || tag === 'textarea') return (await el.inputValue()).trim()
  return ((await el.textContent()) ?? '').trim()
}

/** Primary swap panel in main (heading "Swap" is inside it). */
export function swapActionPanel(page: Page) {
  return page.locator('main .shell-panel-strong').first()
}

async function waitForSwapQuoteIdle(panel: Locator): Promise<void> {
  await expect(async () => {
    // #484 Calculating… / #485 Searching x of y… — both mean quote still in flight.
    const calculating = panel.getByRole('button', { name: /^(Calculating|Searching)/ })
    expect(await calculating.count()).toBe(0)
  }).toPass({ timeout: 120_000 })
}

export async function closeSwapSettingsIfOpen(page: Page): Promise<void> {
  const settings = page.locator('#swap-slippage-settings')
  if (await settings.isVisible().catch(() => false)) {
    await swapActionPanel(page).getByRole('button', { name: 'Settings' }).click()
  }
}

/**
 * Tax/EMBER seed LP vs hub fair-rate often trips the 30% expected-slippage gate
 * (#625 leftover #2). Expert Mode is the LocalTerra escape hatch — not hybrid-off.
 */
export async function enableExpertModeForSwap(page: Page): Promise<void> {
  const panel = swapActionPanel(page)
  const settings = page.locator('#swap-slippage-settings')
  if (!(await settings.isVisible().catch(() => false))) {
    await panel.getByRole('button', { name: 'Settings' }).click()
    await expect(settings).toBeVisible()
  }
  const enableExpert = page.getByTestId('swap-enable-expert-mode')
  if (await enableExpert.isVisible().catch(() => false)) {
    await enableExpert.click()
    await page.getByTestId('expert-mode-confirm-input').fill('ENABLE EXPERT MODE')
    await page.getByTestId('expert-mode-confirm-enable').click()
    await closeSwapSettingsIfOpen(page)
    return
  }
  const expert = page.getByTestId('swap-expert-mode-toggle')
  if (await expert.count()) {
    if (!(await expert.isChecked())) {
      await expert.click({ force: true })
      const confirm = page.getByTestId('expert-mode-confirm-input')
      if (await confirm.isVisible().catch(() => false)) {
        await confirm.fill('ENABLE EXPERT MODE')
        await page.getByTestId('expert-mode-confirm-enable').click()
      }
    }
  }
  await closeSwapSettingsIfOpen(page)
}

/** Native-wrap multihop routes often exceed 1% preset slippage on LocalTerra seed pools. */
export async function openSwapSettingsAndSetSlippage(page: Page, percent: number): Promise<void> {
  const panel = swapActionPanel(page)
  await panel.getByRole('button', { name: 'Settings' }).click()
  const settings = page.locator('#swap-slippage-settings')
  await expect(settings).toBeVisible()
  await settings.getByRole('textbox', { name: /Custom slippage protection/i }).fill(String(percent))
  await waitForSwapQuoteIdle(panel)
  await closeSwapSettingsIfOpen(page)
}

/** Expand Swap Settings → Advanced (integrator controls, GitLab #413). */
export async function expandSwapAdvancedSettings(page: Page): Promise<void> {
  const toggle = page.getByTestId('swap-advanced-settings-toggle')
  if ((await toggle.getAttribute('aria-expanded')) !== 'true') {
    await toggle.click()
  }
  await expect(page.getByTestId('swap-indexer-route-check')).toBeVisible({ timeout: 15_000 })
}

/** Open Settings and expand Advanced for hybrid book leg / indexer route check. */
export async function openSwapAdvancedSettings(page: Page): Promise<void> {
  const panel = swapActionPanel(page)
  await panel.getByRole('button', { name: 'Settings' }).click()
  await expect(page.locator('#swap-slippage-settings')).toBeVisible()
  await expandSwapAdvancedSettings(page)
}

/** Click Swap / Confirm Swap (two-step when price impact is high). */
export async function clickSwapSubmit(page: Page, panel = swapActionPanel(page)) {
  await waitForSwapQuoteIdle(panel)

  const swapAction = panel.getByRole('button').filter({ hasText: /^(Swap|Confirm Swap)/ })
  if ((await swapAction.count()) === 0) {
    const fallback = panel
      .getByRole('button')
      .filter({ hasText: /Swap|Confirm|Route|Connect|Insufficient|Rate Limit|Price impact|Enter Amount|Calculating/i })
      .first()
    assertSwapCtaNotBlocked(await fallback.textContent())
    expect(await swapAction.count(), 'expected Swap or Confirm Swap CTA').toBeGreaterThan(0)
  }

  await expect(swapAction).toBeEnabled({ timeout: 30_000 })
  assertSwapCtaNotBlocked(await swapAction.textContent())
  await swapAction.click()
  await page.waitForTimeout(500)
  const confirmSwap = panel.getByRole('button').filter({ hasText: /^Confirm Swap/ })
  if (await confirmSwap.isVisible().catch(() => false)) {
    await confirmSwap.click()
  }
}
