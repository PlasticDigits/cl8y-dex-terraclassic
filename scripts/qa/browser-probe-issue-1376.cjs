#!/usr/bin/env node
// Fresh disconnected read-only replay; records public bundle/quote evidence only.
const { createRequire } = require('node:module')
const { resolve } = require('node:path')
const requireFrontend = createRequire(resolve(__dirname, '../../frontend-dapp/package.json'))
const { chromium } = requireFrontend('@playwright/test')

async function main() {
  const base = process.env.PROBE_1376_BASE_URL || 'https://dex.cl8y.com/'
  const expectFixed = process.argv.includes('--expect-fixed')
  const matrix = process.argv.includes('--matrix')
  const cases = matrix
    ? ['/', '/swap'].flatMap(path => ['compact', 'full'].flatMap(display => ['10', '100'].flatMap(target =>
        ['cold', 'warm'].map(cache => ({ path, display, target, cache })))))
    : ['10', '100'].map(target => ({ path: new URL(base).pathname, display: 'compact', target, cache: 'cold' }))
  const browser = await chromium.launch({ headless: true })
  const results = []
  try {
    for (const { path, display, target, cache } of cases) {
      const context = await browser.newContext()
      if (display === 'full') await context.addInitScript(() => localStorage.setItem('cl8y-dex-number-display-mode', 'full'))
      const page = await context.newPage()
      const record = { target, path, display, cache, bundles: [], sizing: [], forwards: [], reverseErrors: [] }
      page.on('response', async response => {
        const url = new URL(response.url())
        if (url.pathname.endsWith('.js')) record.bundles.push(url.pathname)
        if (url.pathname.endsWith('/route/solve')) {
          const entry = { amount_in: url.searchParams.get('amount_in'), amount_out: url.searchParams.get('amount_out'), status: response.status() }
          try {
            const quote = await response.json()
            for (const key of ['token_in', 'token_out', 'required_amount_in', 'target_amount_out', 'estimated_amount_out', 'estimated_amount_out_net', 'solver_version', 'sizing_mode', 'fidelity_check', 'snapshot_age_ms', 'snapshot_block_height', 'router_operations']) {
              if (quote[key] !== undefined) entry[key] = quote[key]
            }
          } catch {}
          record.sizing.push(entry)
        }
        if (url.pathname.includes('/smart/')) {
          try {
            const query = JSON.parse(Buffer.from(decodeURIComponent(url.pathname.split('/smart/')[1]), 'base64').toString())
            if (response.status() >= 400 && (query.hybrid_reverse_simulation || query.reverse_simulate_swap_operations)) {
              record.reverseErrors.push({ status: response.status(), message: (await response.json()).message })
            }
            if (response.ok() && query.simulate_swap_operations) {
              const quote = await response.json()
              record.forwards.push({ offer_amount: query.simulate_swap_operations.offer_amount,
                operations: query.simulate_swap_operations.operations, amount: quote.data?.amount })
            }
          } catch {}
        }
      })
      const url = new URL(base)
      url.pathname = path
      url.search = new URLSearchParams({ from: 'USTC', to: 'CL8Y', exactAmount: target, exactField: 'output' }).toString()
      await page.goto(url.toString(), { waitUntil: 'domcontentloaded', timeout: 30000 })
      if (cache === 'warm') {
        await page.getByTestId('swap-you-pay-amount').waitFor({ timeout: 30000 })
        await page.waitForFunction(() => Boolean(document.querySelector('[data-testid="swap-you-pay-amount"]')?.value), null, { timeout: 80000 }).catch(() => {})
        record.sizing.length = 0
        record.forwards.length = 0
        await page.reload({ waitUntil: 'domcontentloaded', timeout: 30000 })
      }
      // A finite observation window; no wallet connection or transaction controls are clicked.
      if (expectFixed) {
        await page.waitForFunction(() => {
          const pay = document.querySelector('[data-testid="swap-you-pay-amount"]')
          return Boolean(pay?.value) || Boolean(document.querySelector('[data-testid="swap-receive-quote-retry-error"]'))
        }, null, { timeout: 80000 }).catch(() => {})
      } else await page.waitForTimeout(25000)
      record.url = url.toString()
      record.pay = await page.getByTestId('swap-you-pay-amount').inputValue().catch(() => null)
      record.receive = await page.getByTestId('swap-you-receive').inputValue().catch(() => null)
      const positivePay = /^\d+(\.\d+)?$/.test(record.pay || '') && /[1-9]/.test(record.pay)
      const matched = record.sizing.some(q => q.amount_out && q.status === 200 && q.required_amount_in &&
        BigInt(q.estimated_amount_out_net || '0') >= BigInt(q.amount_out) &&
        record.forwards.some(f => f.offer_amount === q.required_amount_in && f.operations?.length === q.router_operations?.length &&
          f.operations.every((op, i) => {
            const actual = op.terra_swap, selected = q.router_operations[i].terra_swap
            return actual && selected && JSON.stringify([actual.offer_asset_info, actual.ask_asset_info, actual.hybrid || null]) ===
              JSON.stringify([selected.offer_asset_info, selected.ask_asset_info, selected.hybrid || null])
          })))
      record.status = positivePay && record.receive === target && matched ? 'PASS' : 'FAIL'
      results.push(record)
      await context.close()
    }
  } finally { await browser.close() }
  console.log(JSON.stringify({ utc: new Date().toISOString(), probe_only: true, results }, null, 2))
  if (expectFixed && results.some(row => row.status !== 'PASS')) process.exitCode = 1
}
main().catch(error => { console.error(error.message); process.exitCode = 2 })
