import { expect, type Locator, type Page } from '@playwright/test'

/** LocalTerra dev wallet (simulated wallet mnemonic). */
export const E2E_DEV_WALLET = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'

export function decodeSmartQueryFromUrl(url: string): Record<string, unknown> | null {
  const seg = decodeURIComponent(url.split('/smart/')[1]?.split(/[?#]/)[0] ?? '')
  try {
    return JSON.parse(Buffer.from(seg, 'base64').toString('utf8')) as Record<string, unknown>
  } catch {
    return null
  }
}

function decodeSmartData<T>(raw: { data?: T | string }): T | null {
  const data = raw.data
  if (data == null) return null
  if (typeof data === 'string') {
    try {
      return JSON.parse(Buffer.from(data, 'base64').toString('utf8')) as T
    } catch {
      return null
    }
  }
  return data as T
}

export type HybridSimQuoteCapture = {
  pairAddr: string
  returnAmount: string
  trader?: string
}

export type RouterSimQuoteCapture = {
  returnAmount: string
  trader?: string
}

/** Waits for router `simulate_swap_operations` (Swap page indexer multihop quotes). */
export async function captureRouterSimulateQuote(
  page: Page,
  opts: { requireTrader?: string; timeoutMs?: number } = {}
): Promise<RouterSimQuoteCapture> {
  const timeout = opts.timeoutMs ?? 120_000
  const resp = await page.waitForResponse(
    (r) => {
      if (r.request().method() !== 'GET' || !r.ok()) return false
      const q = decodeSmartQueryFromUrl(r.url())
      if (!q || !('simulate_swap_operations' in q)) return false
      const sim = q.simulate_swap_operations as Record<string, unknown> | undefined
      const trader = sim?.trader as string | undefined
      if (opts.requireTrader && trader !== opts.requireTrader) return false
      return true
    },
    { timeout }
  )
  const body = (await resp.json()) as { data?: { amount?: string } | string }
  const decoded = decodeSmartData<{ amount?: string }>(body)
  const returnAmount = decoded?.amount ?? ''
  expect(returnAmount, 'simulate_swap_operations should return amount').not.toBe('')
  const q = decodeSmartQueryFromUrl(resp.url())
  const trader = (q?.simulate_swap_operations as Record<string, unknown> | undefined)?.trader as string | undefined
  return { returnAmount, trader }
}

/** Waits for a pair `hybrid_simulation` LCD response after the UI triggers a quote. */
export async function captureHybridSimulationQuote(
  page: Page,
  opts: { requireTrader?: string; timeoutMs?: number } = {}
): Promise<HybridSimQuoteCapture> {
  const timeout = opts.timeoutMs ?? 120_000
  const resp = await page.waitForResponse(
    (r) => {
      if (r.request().method() !== 'GET' || !r.ok()) return false
      const q = decodeSmartQueryFromUrl(r.url())
      if (!q || !('hybrid_simulation' in q)) return false
      const sim = q.hybrid_simulation as Record<string, unknown> | undefined
      const trader = sim?.trader as string | undefined
      if (opts.requireTrader && trader !== opts.requireTrader) return false
      return true
    },
    { timeout }
  )
  const url = resp.url()
  const pairAddr = url.split('/contract/')[1]?.split('/smart/')[0] ?? ''
  const body = (await resp.json()) as { data?: { return_amount?: string } | string }
  const decoded = decodeSmartData<{ return_amount?: string }>(body)
  const returnAmount = decoded?.return_amount ?? ''
  expect(returnAmount, 'hybrid_simulation should return return_amount').not.toBe('')
  const q = decodeSmartQueryFromUrl(url)
  const trader = (q?.hybrid_simulation as Record<string, unknown> | undefined)?.trader as string | undefined
  return { pairAddr, returnAmount, trader }
}

/** Submit a tx CTA and wait for success (retries LocalTerra sequence / gas estimate races). */
export async function submitTxButtonWithRetry(
  page: Page,
  submit: Locator,
  opts: { successScope?: Locator; timeoutMs?: number } = {}
): Promise<Locator> {
  const scope = opts.successScope ?? page
  const successAlert = scope.locator('.alert-success').first()
  const errorAlert = scope.locator('.alert-error').first()
  const timeout = opts.timeoutMs ?? 180_000

  let attempt = 0
  await expect(async () => {
    attempt += 1
    if (await successAlert.isVisible().catch(() => false)) {
      await expect(successAlert).toContainText(/TX:/i)
      return
    }
    await expect(submit).toBeEnabled({ timeout: 30_000 })
    await submit.click()
    await page.waitForTimeout(500)

    try {
      await expect(successAlert).toContainText(/TX:/i, { timeout: 45_000 })
      return
    } catch {
      const msg = (await errorAlert.textContent().catch(() => '')) ?? ''
      if (/account sequence mismatch|more gas than estimated/i.test(msg)) {
        await page.waitForTimeout(Math.min(attempt * 2_000, 12_000))
        throw new Error(`retry tx submit (${msg.trim().slice(0, 80)})`)
      }
      if (msg.trim()) throw new Error(`tx submit failed: ${msg.trim()}`)
      throw new Error('no tx result after submit')
    }
  }).toPass({ timeout })

  return successAlert
}
