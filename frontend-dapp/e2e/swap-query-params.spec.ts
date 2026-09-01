import { test, expect } from '@playwright/test'
import {
  ARIA_SELECT_TOKEN_PAY,
  ARIA_SELECT_TOKEN_RECEIVE,
  payTokenTrigger,
  receiveTokenTrigger,
  waitForPayTokenTriggerEnabled,
} from './helpers/token-select'

/**
 * GitLab #711 — Swap deep-link query params. Needs factory tokens (same as #498/#632).
 * PLAYWRIGHT_SKIP_CHAIN=1 is OK when LCD still serves pairs.
 */
test.describe('Swap query params (GitLab #711)', () => {
  test('/swap without query still lands on Swap UI', async ({ page }) => {
    await page.goto('/swap')
    await page.waitForLoadState('networkidle')
    await waitForPayTokenTriggerEnabled(page, 90_000)
    await expect(page.getByRole('heading', { name: /^swap$/i })).toBeVisible()
    expect(new URL(page.url()).pathname).toMatch(/^\/(swap\/?)?$/)
  })

  test('/swap?from=uluna&to=uusd preserves pair after redirect', async ({ page }) => {
    await page.goto('/swap?from=uluna&to=uusd')
    await page.waitForLoadState('networkidle')
    await waitForPayTokenTriggerEnabled(page, 90_000)
    await expect(payTokenTrigger(page)).toContainText(/LUNC/i)
    await expect(receiveTokenTrigger(page)).toContainText(/USTC/i)
    const url = new URL(page.url())
    expect(url.pathname).toBe('/')
    expect(url.search).toMatch(/from=LUNC/)
    expect(url.search).toMatch(/to=USTC/)
    await expect(page.getByRole('link', { name: 'Swap' }).first()).toHaveAttribute('aria-current', 'page')
  })

  test('/?from=uluna&to=uusd selects LUNC / USTC', async ({ page }) => {
    await page.goto('/?from=uluna&to=uusd')
    await page.waitForLoadState('networkidle')
    await waitForPayTokenTriggerEnabled(page, 90_000)
    await expect(payTokenTrigger(page)).toContainText(/LUNC/i)
    await expect(receiveTokenTrigger(page)).toContainText(/USTC/i)
  })

  test('Uniswap outputCurrency only sets receive', async ({ page }) => {
    await page.goto('/?outputCurrency=uusd')
    await page.waitForLoadState('networkidle')
    await waitForPayTokenTriggerEnabled(page, 90_000)
    await expect(receiveTokenTrigger(page)).toContainText(/USTC/i)
    await expect(payTokenTrigger(page)).not.toContainText(/USTC/i)
  })

  test('hostile from= is ignored and not echoed', async ({ page }) => {
    await page.goto('/?from=javascript:alert(1)&to=uusd')
    await page.waitForLoadState('networkidle')
    await waitForPayTokenTriggerEnabled(page, 90_000)
    const pay = payTokenTrigger(page)
    await expect(pay).not.toContainText(/javascript/i)
    await expect(page.locator(`[aria-label="${ARIA_SELECT_TOKEN_PAY}"]`)).toBeVisible()
    await expect(page.locator(`[aria-label="${ARIA_SELECT_TOKEN_RECEIVE}"]`)).toBeVisible()
  })
})
