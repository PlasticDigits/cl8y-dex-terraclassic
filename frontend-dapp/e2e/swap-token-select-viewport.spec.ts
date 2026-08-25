import { test, expect } from '@playwright/test'
import {
  ARIA_SELECT_TOKEN_PAY,
  ARIA_SELECT_TOKEN_RECEIVE,
  payTokenTrigger,
  receiveTokenTrigger,
  waitForPayTokenTriggerEnabled,
} from './helpers/token-select'

const TAB_BAR_FALLBACK_PX = 56
const FINGER_GAP_PX = 44

/**
 * GitLab #632 — token listbox stays above the reserved bottom band on a phone
 * viewport. No LocalTerra txs. Needs factory tokens (same as #498 CLS smoke).
 */
test.describe('Swap token select visual viewport clearance (GitLab #632)', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      const vv = {
        width: 390,
        height: 400,
        offsetTop: 0,
        offsetLeft: 0,
        scale: 1,
        onresize: null,
        onscroll: null,
        addEventListener() {},
        removeEventListener() {},
        dispatchEvent() {
          return false
        },
      }
      Object.defineProperty(window, 'visualViewport', { configurable: true, value: vv })
    })
    await page.setViewportSize({ width: 390, height: 844 })
  })

  async function assertListAboveReserved(page: import('@playwright/test').Page, ariaLabel: string) {
    const list = page.getByRole('listbox', { name: ariaLabel })
    await expect(list).toBeVisible({ timeout: 10_000 })
    const clearance = await page.evaluate(
      ({ tabFallback, finger }) => {
        const menu = document.querySelector('.token-select-dropdown') as HTMLElement | null
        const nav = document.querySelector<HTMLElement>('.app-mobile-nav-shell')
        const tab =
          nav && getComputedStyle(nav).display !== 'none' ? Math.ceil(nav.getBoundingClientRect().height) : tabFallback
        const box = menu?.getBoundingClientRect()
        return {
          bottom: box?.bottom ?? -1,
          reserved: window.innerHeight - tab - finger,
          href: location.pathname,
        }
      },
      { tabFallback: TAB_BAR_FALLBACK_PX, finger: FINGER_GAP_PX }
    )
    expect(clearance.bottom).toBeGreaterThan(0)
    expect(clearance.bottom).toBeLessThanOrEqual(clearance.reserved + 1)
    // Swap is the site root (`/`); `/swap` redirects there.
    expect(clearance.href).toMatch(/^\/(swap\/?)?$/)
  }

  test('pay listbox stays above tab bar + finger gap; last option click stays on /swap', async ({ page }) => {
    await page.goto('/swap')
    await page.waitForLoadState('networkidle')
    await waitForPayTokenTriggerEnabled(page, 90_000)

    await payTokenTrigger(page).click()
    await assertListAboveReserved(page, ARIA_SELECT_TOKEN_PAY)

    const list = page.getByRole('listbox', { name: ARIA_SELECT_TOKEN_PAY })
    const options = list.getByRole('option')
    await expect(async () => {
      expect(await options.count()).toBeGreaterThan(0)
    }).toPass({ timeout: 10_000 })
    const last = options.last()
    const label = (await last.innerText()).replace(/\s+/g, ' ').trim()
    const tokenHint = label.split(/\s+/)[0] ?? label
    await last.click()
    await expect(page.locator('.token-select-dropdown')).toHaveCount(0)
    const trigger = payTokenTrigger(page)
    const shown = await trigger.evaluate((el) => (el instanceof HTMLInputElement ? el.value : (el.textContent ?? '')))
    expect(shown.replace(/\s+/g, ' ')).toContain(tokenHint)
    expect(new URL(page.url()).pathname).toMatch(/^\/(swap\/?)?$/)
  })

  test('receive listbox stays above the reserved band', async ({ page }) => {
    await page.goto('/swap')
    await page.waitForLoadState('networkidle')
    await waitForPayTokenTriggerEnabled(page, 90_000)
    await expect(receiveTokenTrigger(page)).toBeEnabled({ timeout: 25_000 })

    await receiveTokenTrigger(page).click()
    await assertListAboveReserved(page, ARIA_SELECT_TOKEN_RECEIVE)
    await page.keyboard.press('Escape')
  })
})
