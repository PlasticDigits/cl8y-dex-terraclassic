import { test, expect, type APIRequestContext, type Page } from '@playwright/test'

const DEFAULT_LCD = 'http://localhost:1317'

/**
 * Optional chain mode for UI-only local dev (not default CI).
 * - `PLAYWRIGHT_SKIP_CHAIN=1` (preferred; GitLab #201)
 * - `REQUIRE_LOCALTERRA=0` (legacy alias)
 */
export function isChainOptional(): boolean {
  return process.env.PLAYWRIGHT_SKIP_CHAIN === '1' || process.env.REQUIRE_LOCALTERRA === '0'
}

/** @deprecated Use `isChainOptional()` */
export function isLocalTerraOptional(): boolean {
  return isChainOptional()
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

/** Ensures LCD is up on strict paths; skips the test only when chain is optional. */
export async function skipIfLcdUnreachable(request: APIRequestContext): Promise<void> {
  if (isChainOptional()) {
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

/** Strict tx specs: success or error alert must appear (GitLab #103 / #201). */
export async function assertTxResultAlert(page: Page, timeoutMs = 90_000): Promise<void> {
  const alert = page.locator('.alert-success, .alert-error').first()
  await expect(alert, 'expected tx success or error alert after submit').toBeVisible({ timeout: timeoutMs })
}

/**
 * Optional-chain legacy helper — prefer `assertTxResultAlert` in `e2e-tx` specs.
 * Skips when no alert in optional mode; fails in strict mode.
 */
export async function skipIfNoTxAlert(page: Page, timeoutMs = 90_000): Promise<void> {
  if (isChainOptional()) {
    const alert = page.locator('.alert-success, .alert-error')
    try {
      await alert.waitFor({ state: 'visible', timeout: timeoutMs })
    } catch {
      test.skip(
        true,
        'No tx result alert; ensure LocalTerra matches VITE_* addresses, contracts are deployed, and the dev account is funded.'
      )
    }
    return
  }
  await assertTxResultAlert(page, timeoutMs)
}

/** Strict pool / liquidity txs: primary CTA must not be blocked after globalSetup funding. */
export function assertLiquidityCtaNotBlocked(label: string | null, detail: string): void {
  expect(label, detail).not.toMatch(/Insufficient|Connect/i)
}

const SWAP_CTA_MSG =
  'Swap CTA blocked after E2E provisioning; fund dev wallet (scripts/e2e-provision-dev-wallet.sh) and ensure router/pools are deployed (GitLab #201).'

/** Strict swap card: must not show Insufficient Balance / No Route / Connect after global setup. */
export function assertSwapCtaNotBlocked(label: string | null, detail = SWAP_CTA_MSG): void {
  expect(label, detail).not.toMatch(/Insufficient Balance|No Route|Connect|Rate Limit|Price impact/i)
}
