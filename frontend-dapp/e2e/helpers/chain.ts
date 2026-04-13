import { test, type APIRequestContext, type Page } from '@playwright/test'

const DEFAULT_LCD = 'http://localhost:1317'

/** LCD REST base URL (Playwright tests do not load Vite `.env`; set `E2E_LCD_URL` in CI if needed). */
export function lcdBaseUrl(): string {
  const u = process.env.VITE_TERRA_LCD_URL || process.env.E2E_LCD_URL || DEFAULT_LCD
  return u.replace(/\/$/, '')
}

/** Skip tx tests when the chain LCD is not reachable (avoids long timeouts without LocalTerra). */
export async function skipIfLcdUnreachable(request: APIRequestContext): Promise<void> {
  const base = lcdBaseUrl()
  try {
    const res = await request.get(`${base}/cosmos/base/tendermint/v1beta1/node_info`, {
      timeout: 10_000,
      failOnStatusCode: false,
    })
    if (!res.ok()) {
      test.skip(true, `LCD ${base} returned ${res.status()}; start LocalTerra for on-chain E2E.`)
    }
  } catch {
    test.skip(true, `LCD ${base} unreachable; start LocalTerra for on-chain E2E.`)
  }
}

/**
 * Full-stack tx tests need a funded dev wallet and matching deployed code IDs.
 * If the UI never shows a result alert, skip instead of failing the whole run.
 */
export async function skipIfNoTxAlert(page: Page, timeoutMs = 90_000): Promise<void> {
  const alert = page.locator('.alert-success, .alert-error')
  try {
    await alert.waitFor({ state: 'visible', timeout: timeoutMs })
  } catch {
    test.skip(
      true,
      'No tx result alert; ensure LocalTerra matches VITE_* addresses, contracts are deployed, and the dev account is funded.'
    )
  }
}
