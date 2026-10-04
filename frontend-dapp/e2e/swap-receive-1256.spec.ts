import { test, expect, type Page } from '@playwright/test'
import { SWAP_RECEIVE_1256_FIXTURES as fixture } from './helpers/swap-receive-env'

const UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
const USTR = 'terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv'
const CL8Y = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const decimals = (token: string) => (token === USTR || token === CL8Y ? 18 : 6)
type TokenInfo = { token: { contract_addr: string } }
const rawUnit = (info: TokenInfo) => 10n ** BigInt(decimals(info.token.contract_addr))

const pairs = [
  [fixture.clunc, fixture.custc],
  [USTR, UST1],
  [CL8Y, USTR],
].map(([a, b], i) => ({
  contract_addr: `terra1pair1256000000000000000000000000000${i}`,
  liquidity_token: `terra1lp12560000000000000000000000000000${i}`,
  asset_infos: [{ token: { contract_addr: a } }, { token: { contract_addr: b } }],
}))

/** LCD-only fixture: these checks exercise real browser inputs, routing and reverse services. */
async function mockSwapLcd(page: Page) {
  const asks: { amount: string; kind: 'pair' | 'router'; hops: number }[] = []
  await page.route('**/*', async (route) => {
    const url = new URL(route.request().url())
    const smart = url.pathname.match(/\/contract\/([^/]+)\/smart\/([^/]+)/)
    const fulfill = (body: unknown, status = 200) =>
      route.fulfill({
        status,
        contentType: 'application/json',
        body: JSON.stringify(body),
      })
    if (smart) {
      const query = JSON.parse(Buffer.from(decodeURIComponent(smart[2]), 'base64').toString('utf8'))
      let data: unknown = {}
      if (query.pairs) data = { pairs }
      else if (query.pair) {
        const ids = query.pair.asset_infos?.map(
          (info: { token: { contract_addr: string } }) => info.token.contract_addr
        )
        data = {
          pair:
            pairs.find((pair) => pair.asset_infos.every((info) => ids?.includes(info.token.contract_addr))) ?? pairs[0],
        }
      } else if (query.config) {
        data = {
          governance: fixture.factory,
          treasury: fixture.treasury,
          paused: false,
          fee_wrap_bps: 0,
          fee_unwrap_bps: 0,
        }
      } else if (query.token_info) {
        const symbols: Record<string, string> = {
          [fixture.clunc]: 'cLUNC',
          [fixture.custc]: 'cUSTC',
          [UST1]: 'UST1',
          [USTR]: 'USTR',
          [CL8Y]: 'CL8Y',
        }
        data = {
          name: symbols[smart[1]] ?? 'Fixture',
          symbol: symbols[smart[1]] ?? 'FIX',
          decimals: decimals(smart[1]),
          total_supply: '1000000000000000000000000',
        }
      } else if (query.reverse_simulate_swap_operations) {
        const ask = query.reverse_simulate_swap_operations.ask_amount
        const operations = query.reverse_simulate_swap_operations.operations
        asks.push({ amount: ask, kind: 'router', hops: operations.length })
        data = {
          amount: (
            (BigInt(ask) * 2n * rawUnit(operations[0].terra_swap.offer_asset_info)) /
            rawUnit(operations.at(-1).terra_swap.ask_asset_info)
          ).toString(),
        }
      } else if (query.hybrid_reverse_simulation) {
        const ask = query.hybrid_reverse_simulation.ask_asset.amount
        asks.push({ amount: ask, kind: 'pair', hops: 1 })
        const askInfo = query.hybrid_reverse_simulation.ask_asset.info
        const pair = pairs.find((pair) => pair.contract_addr === smart[1])!
        const offerInfo = pair.asset_infos.find((info) => info.token.contract_addr !== askInfo.token.contract_addr)!
        data = {
          offer_amount: ((BigInt(ask) * 2n * rawUnit(offerInfo)) / rawUnit(askInfo)).toString(),
          spread_amount: '0',
          commission_amount: '0',
        }
      } else if (query.simulate_swap_operations) {
        const operations = query.simulate_swap_operations.operations
        data = {
          amount: (
            (BigInt(query.simulate_swap_operations.offer_amount) *
              rawUnit(operations.at(-1).terra_swap.ask_asset_info)) /
            rawUnit(operations[0].terra_swap.offer_asset_info) /
            2n
          ).toString(),
        }
      } else if (query.hybrid_simulation) {
        const offerInfo = query.hybrid_simulation.offer_asset.info
        const pair = pairs.find((pair) => pair.contract_addr === smart[1])!
        const askInfo = pair.asset_infos.find((info) => info.token.contract_addr !== offerInfo.token.contract_addr)!
        const amount = (
          (BigInt(query.hybrid_simulation.offer_asset.amount) * rawUnit(askInfo)) /
          rawUnit(offerInfo) /
          2n
        ).toString()
        data = {
          return_amount: amount,
          pool_return_amount: amount,
          book_return_amount: '0',
          spread_amount: '0',
          commission_amount: '0',
          pool_commission_amount: '0',
          book_commission_amount: '0',
        }
      } else if (query.pool) {
        const pair = pairs.find((pair) => pair.contract_addr === smart[1]) ?? pairs[0]
        data = {
          assets: pair.asset_infos.map((info) => ({ info, amount: '1000000000000' })),
          total_share: '1000000000000',
        }
      } else if (query.balance) data = { balance: '1000000000' }
      else if (query.blacklist_check) data = { blocked: false }
      else if (query.get_fee_config) data = { fee_config: { fee_bps: 30, treasury: fixture.treasury } }
      else if (query.rate_limit)
        data = {
          config: { max_amount_per_window: '1000000000000', window_seconds: 86400 },
          amount_used: '0',
          current_window_start: null,
        }
      else if (query.is_paused) data = { paused: false }
      else if (query.is_code_id_whitelisted) data = { whitelisted: true, code_id: 1 }
      return fulfill({ data })
    }
    if (url.pathname.includes('/terra/tax/v1beta1/params')) return fulfill({ params: { burn_tax_rate: '0' } })
    if (url.pathname.includes('/tax_caps/')) return fulfill({ tax_cap: '1000000000000' })
    if (url.pathname.includes('/cosmos/bank/'))
      return fulfill({
        balances: [
          { denom: 'uluna', amount: '1000000000' },
          { denom: 'uusd', amount: '1000000000' },
        ],
      })
    // Force the client route fallback; GET /route/solve has only an offer-side API.
    if (url.pathname.includes('/route/solve') || url.pathname.startsWith('/api/'))
      return fulfill({ error: 'fixture indexer offline' }, 503)
    return route.continue()
  })
  return asks
}

async function openSwap(page: Page, search = '?from=LUNC&to=USTC') {
  await page.goto(`/${search}`)
  await expect(page.getByRole('combobox', { name: 'Select token you pay' })).toBeEnabled({ timeout: 20_000 })
  await expect(page.getByTestId('swap-you-receive')).toHaveAttribute('inputmode', 'decimal')
}

test.describe('Receive amount editing (#1256)', () => {
  let reverseRequests: Awaited<ReturnType<typeof mockSwapLcd>>
  test.beforeEach(async ({ page }) => {
    reverseRequests = await mockSwapLcd(page)
  })

  test('default Swap always renders the receive amount as an editable decimal input', async ({ page }) => {
    await openSwap(page, '')
    await expect(page.getByTestId('swap-you-receive')).toBeEditable()
    await expect(page.getByTestId('swap-you-receive')).toHaveAttribute('maxlength', '24')
  })

  test('native pay to a listed wrapped hub accepts the receive target', async ({ page }) => {
    await openSwap(page, `?from=uluna&to=${fixture.custc}`)
    await expect(page.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('LUNC')
    await expect(page.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('cUSTC')
    await page.getByTestId('swap-you-receive').fill('1.5')
    await expect(page.getByTestId('swap-you-pay-amount')).toHaveValue('3.000')
    await expect(page.getByTestId('swap-you-receive')).toHaveValue('1.5')
    await expect(page).toHaveURL(/exactAmount=1.5.*exactField=output/)
    expect(reverseRequests).toContainEqual({ amount: '1500000', kind: 'router', hops: 1 })
  })

  test('native receive edits update pay and canonical URL without growing history', async ({ page }) => {
    await openSwap(page)
    const historyLength = await page.evaluate(() => history.length)
    const receive = page.getByTestId('swap-you-receive')
    await receive.fill('1.5')
    await expect(receive).toHaveValue('1.5')
    await expect(page.getByTestId('swap-you-pay-amount')).toHaveValue('3.000')
    expect(reverseRequests).toContainEqual({ amount: '1500000', kind: 'router', hops: 1 })
    await expect(page).toHaveURL(/exactAmount=1.5.*exactField=output/)
    await receive.fill('2')
    await expect(page.getByTestId('swap-you-pay-amount')).toHaveValue('4.000')
    expect(await page.evaluate(() => history.length)).toBe(historyLength)
    await expect(page.getByTestId('swap-submit')).toBeEnabled()
    await expect(page.getByTestId('swap-submit')).toHaveText('Connect Wallet')
  })

  test('direct CW20 output deep link and typing quote the required pay amount', async ({ page }) => {
    await openSwap(page, `?from=${USTR}&to=${UST1}&exactField=output&exactAmount=1.5`)
    await expect(page.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('USTR')
    await expect(page.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('UST1')
    await expect(page.getByTestId('swap-you-receive')).toHaveValue('1.5')
    await expect(page.getByTestId('swap-you-pay-amount')).toHaveValue('3.000')
    expect(reverseRequests).toContainEqual({ amount: '1500000', kind: 'pair', hops: 1 })
    await page.getByTestId('swap-you-receive').fill('2')
    await expect(page.getByTestId('swap-you-pay-amount')).toHaveValue('4.000')
  })

  test('CW20 multihop receive quote works with the indexer unavailable', async ({ page }) => {
    await openSwap(page, `?from=${CL8Y}&to=${UST1}`)
    await expect(page.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('CL8Y')
    await expect(page.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('UST1')
    await page.getByTestId('swap-you-receive').fill('1.5')
    await expect(page.getByTestId('swap-you-pay-amount')).toHaveValue('3.000')
    await expect(page.getByTestId('swap-you-receive')).toHaveValue('1.5')
    expect(reverseRequests).toContainEqual({ amount: '1500000', kind: 'router', hops: 2 })
  })

  for (const [direction, from, to] of [
    ['wrap', 'uluna', fixture.clunc],
    ['unwrap', fixture.clunc, 'uluna'],
  ]) {
    test(`direct ${direction} accepts receive input`, async ({ page }) => {
      await openSwap(page, `?from=${from}&to=${to}`)
      await expect(page.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue(
        direction === 'wrap' ? 'LUNC' : 'cLUNC'
      )
      await expect(page.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue(
        direction === 'wrap' ? 'cLUNC' : 'LUNC'
      )
      await page.getByTestId('swap-you-receive').fill('1.5')
      await expect(page.getByTestId('swap-you-pay-amount')).toHaveValue('1.500')
      await expect(page.getByTestId('swap-you-receive')).toHaveValue('1.5')
      expect(reverseRequests).toHaveLength(0)
    })
  }

  test('keyboard editing, clearing and flip preserve receive ownership', async ({ page }) => {
    await openSwap(page)
    const receive = page.getByTestId('swap-you-receive')
    await receive.focus()
    await receive.pressSequentially('1.5')
    await expect(receive).toHaveValue('1.5')
    await expect(page.getByTestId('swap-you-pay-amount')).toHaveValue('3.000')
    await page.getByRole('button', { name: 'Swap pay and receive tokens' }).click()
    await expect(receive).toHaveValue('1.5')
    await expect(page.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('USTC')
    await receive.fill('')
    await expect(receive).toHaveValue('')
    await expect(page.getByTestId('swap-you-pay-amount')).toHaveValue('')
    await expect(page).not.toHaveURL(/exactAmount=/)
  })

  test('receive sanitization rejects exponent text and caps browser paste', async ({ page }) => {
    await openSwap(page)
    const receive = page.getByTestId('swap-you-receive')
    await receive.fill('1')
    await receive.fill('1e9')
    await expect(receive).toHaveValue('1')
    await expect(receive).toHaveAttribute('maxlength', '24')
    await receive.fill('123456789012345678901234567890')
    expect((await receive.inputValue()).length).toBeLessThanOrEqual(24)
  })
})
