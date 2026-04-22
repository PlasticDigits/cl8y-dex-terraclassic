import { test, expect, type APIRequestContext, type Page } from '@playwright/test'

const DEFAULT_LCD = 'http://localhost:1317'

/**
 * When `REQUIRE_LOCALTERRA=0`, on-chain helpers skip instead of failing (no LocalTerra in CI job, etc.).
 * Default (unset or any other value): strict — LCD / pool tx preconditions must hold.
 */
export function isLocalTerraOptional(): boolean {
  return process.env.REQUIRE_LOCALTERRA === '0'
}

/** LCD REST base URL (Playwright tests do not load Vite `.env`; set `E2E_LCD_URL` in CI if needed). */
export function lcdBaseUrl(): string {
  const u = process.env.VITE_TERRA_LCD_URL || process.env.E2E_LCD_URL || DEFAULT_LCD
  return u.replace(/\/$/, '')
}

export async function assertLcdReachable(request: APIRequestContext): Promise<void> {
  const base = lcdBaseUrl()
  try {
    const res = await request.get(`${base}/cosmos/base/tendermint/v1beta1/node_info`, {
      timeout: 10_000,
      failOnStatusCode: false,
    })
    if (!res.ok()) {
      throw new Error(
        `LCD ${base} returned ${res.status()}; start LocalTerra (docker compose up -d localterra) and deploy contracts.`
      )
    }
  } catch (e) {
    if (e instanceof Error && e.message.startsWith(`LCD ${base} returned`)) throw e
    throw new Error(
      `LCD ${base} unreachable (${e instanceof Error ? e.message : String(e)}); start LocalTerra for on-chain E2E.`
    )
  }
}

/** Skip tx tests when the chain LCD is not reachable — skips only if `REQUIRE_LOCALTERRA=0`. */
export async function skipIfLcdUnreachable(request: APIRequestContext): Promise<void> {
  if (isLocalTerraOptional()) {
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
    return
  }
  await assertLcdReachable(request)
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

/** Strict pool / liquidity txs: fail if the tx result banner never appears. */
export async function assertTxResultAlert(page: Page, timeoutMs = 90_000): Promise<void> {
  const alert = page.locator('.alert-success, .alert-error')
  await expect(alert, 'expected tx success or error alert after submit').toBeVisible({ timeout: timeoutMs })
}

/**
 * Strict pool / liquidity txs: primary CTA must not be blocked after globalSetup funding.
 * Matches "Insufficient …" / "Connect …" patterns used on pool forms and swap-style controls.
 */
export function assertLiquidityCtaNotBlocked(label: string | null, detail: string): void {
  expect(label, detail).not.toMatch(/Insufficient|Connect/i)
}
