import { test, expect } from './fixtures/dev-wallet'
import type { APIRequestContext, Page } from '@playwright/test'
import { readFrontendEnvLocal } from './helpers/community-tax-env'
import { decodeSmartQueryFromUrl } from './helpers/fee-discount-quote-e2e'
import { skipIfLcdUnreachable } from './helpers/chain'
import { clickSwapSubmit, swapActionPanel } from './helpers/swap-ui'
import { fetchTxJson, readTxHashFromAlertLink, txJsonWasmSwapHops, txJsonWasmAttrForAction } from './helpers/lcd'

/** Real LocalTerra/indexer execution. Run serially after deploy/indexer and dev-wallet provisioning. */
test('native receive sizing preserves protection on a dust-sized LocalTerra route (#1376)', async ({
  page,
  connectWallet,
  request,
}) => {
  test.setTimeout(300_000)
  await skipIfLcdUnreachable(request)
  await connectWallet
  // Pin the funded native route explicitly; local retail defaults can prefer seeded gems.
  await page.goto('/?from=LUNC&to=USTC')
  await page.waitForLoadState('networkidle')
  await expect(page.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('LUNC')
  await expect(page.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('USTC')
  const quoteResponse = page.waitForResponse(
    (response) =>
      response.url().includes('/api/v1/route/solve?') && new URL(response.url()).searchParams.has('amount_out'),
    { timeout: 75_000 }
  )
  await page.getByTestId('swap-you-receive').fill('0.001')
  const response = await quoteResponse
  if (response.status() === 422) {
    const reason = await response.text()
    expect(reason).toContain('>=99% symmetric fair-rate protection')
    await expect(page.getByTestId('swap-receive-quote-retry-error')).toBeVisible({ timeout: 75_000 })
    await expect(page.getByTestId('swap-you-pay-amount')).toHaveValue('')
    await expect(page.getByTestId('swap-you-receive')).toHaveValue('0.001')
    await expect(page.getByTestId('swap-submit')).toBeDisabled()
    test.info().annotations.push({
      type: 'native_execution_unavailable',
      description:
        'Native receive intent and server >=99% fair-rate rejection checked; no native execution on the synthetic seed price fixture.',
    })
    return
  }
  expect(response.status(), response.ok() ? undefined : await response.text()).toBe(200)
  const selected = (await response.json()) as {
    required_amount_in: string
    sizing_mode: string
    router_operations: {
      terra_swap: {
        offer_asset_info: { token: { contract_addr: string } }
        ask_asset_info: { token: { contract_addr: string } }
        hybrid?: unknown
      }
    }[]
  }
  expect(selected.sizing_mode).toBe('pool_only_exact_output')
  expect(BigInt(selected.required_amount_in)).toBeGreaterThan(0n)
  expect(selected.router_operations.length).toBeGreaterThan(0)
  expect(selected.router_operations.every((op) => op.terra_swap.hybrid == null)).toBe(true)
  await expect(page.getByTestId('swap-you-pay-amount')).not.toHaveValue('', { timeout: 75_000 })
  await expect(page.getByTestId('swap-you-receive')).toHaveValue('0.001')
  const panel = swapActionPanel(page)
  await expect(async () => {
    expect(
      (await page.getByTestId('swap-submit').isEnabled()) ||
        (await page.getByTestId('swap-slippage-blocked').isVisible())
    ).toBe(true)
  }).toPass({ timeout: 75_000 })
  const risk = page.getByTestId('swap-slippage-blocked')
  if (await risk.isVisible().catch(() => false)) {
    const text = await risk.innerText()
    const percent = Number(text.match(/Expected slippage is ([\d.]+)%/)?.[1])
    expect(Number.isFinite(percent), 'guard must identify the actual fair-rate deviation').toBe(true)
    if (await page.getByTestId('swap-theater-quote-blocked').isVisible()) {
      expect(percent).toBeGreaterThanOrEqual(99)
      await expect(risk).toContainText('Expert Mode cannot waive it')
    } else {
      expect(percent).toBeGreaterThan(30)
      expect(percent).toBeLessThan(99)
      await expect(risk).toContainText('above 30%')
      await expect(risk).toContainText('Enable Expert Mode to continue')
    }
    await expect(page.getByTestId('swap-submit')).toBeDisabled()
    test.info().annotations.push({
      type: 'native_execution_unavailable',
      description: `Native sizing/guard checked; no native execution: synthetic seed prices/rounding produce ${percent}% fair-rate deviation and require blocked risk consent. No Expert or spread override was enabled.`,
    })
    return
  }
  await clickSwapSubmit(page, panel)
  const success = panel.locator('.alert-success').first()
  await expect(success).toBeVisible({ timeout: 120_000 })
  const hash = await readTxHashFromAlertLink(page, success)
  await expect(async () => {
    const tx = await fetchTxJson(request, hash)
    expect(tx).toBeTruthy()
    const expectedHops = selected.router_operations.map((op) => ({
      offerAsset: op.terra_swap.offer_asset_info.token.contract_addr,
      askAsset: op.terra_swap.ask_asset_info.token.contract_addr,
    }))
    expect(txJsonWasmSwapHops(tx)).toEqual(expectedHops)
    // Match the first pool hop offer after the native mapper deposit.
    expect(txJsonWasmAttrForAction(tx, 'swap', 'offer_amount')).toBe(selected.required_amount_in)
  }).toPass({ timeout: 120_000 })
})

type SelectedSizing = {
  required_amount_in: string
  sizing_mode: string
  router_operations: {
    terra_swap: {
      offer_asset_info: { token: { contract_addr: string } }
      ask_asset_info: { token: { contract_addr: string } }
      hybrid?: unknown
    }
  }[]
}

async function assertCw20Receipt(
  page: Page,
  request: APIRequestContext,
  selected: SelectedSizing,
  from: string,
  walletReturn: string
) {
  const panel = swapActionPanel(page)
  await clickSwapSubmit(page, panel)
  const success = panel.locator('.alert-success').first()
  await expect(success).toBeVisible({ timeout: 120_000 })
  const hash = await readTxHashFromAlertLink(page, success)
  await expect(async () => {
    const tx = await fetchTxJson(request, hash)
    expect(tx).toBeTruthy()
    expect(txJsonWasmSwapHops(tx)).toEqual(
      selected.router_operations.map((op) => ({
        offerAsset: op.terra_swap.offer_asset_info.token.contract_addr,
        askAsset: op.terra_swap.ask_asset_info.token.contract_addr,
      }))
    )
    expect(txJsonWasmAttrForAction(tx, 'swap', 'offer_amount')).toBe(selected.required_amount_in)
    const messages =
      (tx as { tx?: { body?: { messages?: { contract?: string; msg?: string | Record<string, unknown> }[] } } }).tx
        ?.body?.messages ?? []
    const cw20 = messages.find((message) => message.contract === from)
    expect(cw20, 'transaction must fund the selected route with the quoted CW20').toBeTruthy()
    const execute =
      typeof cw20!.msg === 'string' ? JSON.parse(Buffer.from(cw20!.msg, 'base64').toString('utf8')) : cw20!.msg
    const send = execute.send as { amount: string; msg: string }
    expect(send.amount).toBe(selected.required_amount_in)
    const hook = JSON.parse(Buffer.from(send.msg, 'base64').toString('utf8')) as {
      swap?: { min_return?: string }
      execute_swap_operations?: { minimum_receive?: string }
    }
    const minimum = hook.swap?.min_return ?? hook.execute_swap_operations?.minimum_receive
    expect(minimum, 'offer-in execution must retain the wallet quote slippage floor').toBe(
      ((BigInt(walletReturn) * 95n) / 100n).toString()
    )
  }).toPass({ timeout: 120_000 })
}

test('funded CW20 receive sizing executes the selected route with matching offer and minimum (#1376)', async ({
  page,
  connectWallet,
  request,
}) => {
  test.setTimeout(300_000)
  await skipIfLcdUnreachable(request)
  await connectWallet
  const local = readFrontendEnvLocal()
  const from = process.env.VITE_TOKEN_EMBER_ADDRESS || local.VITE_TOKEN_EMBER_ADDRESS
  const to = process.env.VITE_TOKEN_CORAL_ADDRESS || local.VITE_TOKEN_CORAL_ADDRESS
  expect(from, 'seeded EMBER contract is required').toBeTruthy()
  expect(to, 'seeded CORAL contract is required').toBeTruthy()
  await page.goto(`/?from=${from}&to=${to}`)
  await page.waitForLoadState('networkidle')
  const walletQuotes: Promise<{ offer: string; amount: string } | null>[] = []
  page.on('response', (response) => {
    if (!response.ok() || !response.url().includes('/smart/')) return
    const decoded = decodeSmartQueryFromUrl(response.url())
    const sim = decoded?.simulate_swap_operations as { offer_amount?: string } | undefined
    if (!sim?.offer_amount) return
    walletQuotes.push(
      response
        .json()
        .then((body) => ({ offer: sim.offer_amount!, amount: body.data.amount }))
        .catch(() => null)
    )
  })
  const pending = page.waitForResponse(
    (response) =>
      response.url().includes('/api/v1/route/solve?') && new URL(response.url()).searchParams.has('amount_out'),
    { timeout: 75_000 }
  )
  // One CORAL (six decimals) avoids the one-raw-unit pool rounding in the native negative fixture.
  await page.getByTestId('swap-you-receive').fill('1')
  const response = await pending
  expect(response.status()).toBe(200)
  const selected = (await response.json()) as SelectedSizing
  expect(selected.sizing_mode).toBe('pool_only_exact_output')
  expect(selected.router_operations.every((op) => op.terra_swap.hybrid == null)).toBe(true)
  await expect(page.getByTestId('swap-you-pay-amount')).not.toHaveValue('', { timeout: 75_000 })
  await expect(page.getByTestId('swap-you-receive')).toHaveValue('1')
  await expect(page.getByTestId('swap-submit')).toBeEnabled({ timeout: 75_000 })
  await expect(page.getByTestId('swap-theater-quote-blocked')).toHaveCount(0)
  const wallet = (await Promise.all(walletQuotes)).findLast((quote) => quote?.offer === selected.required_amount_in)
  expect(wallet, 'selected offer must have an authoritative wallet forward quote').toBeTruthy()
  expect(BigInt(wallet!.amount)).toBeGreaterThanOrEqual(1000000n)
  await assertCw20Receipt(page, request, selected, from!, wallet!.amount)
})
