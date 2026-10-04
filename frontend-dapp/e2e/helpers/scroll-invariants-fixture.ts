import type { Page } from '@playwright/test'
import type { IndexerCandle, IndexerPair } from '../../src/types'

export const SCROLL_PAIR_ADDRESS = 'terra10y4jzxavk0uw2usy7ezt4dq5h0k64na8c9yz3rq3dk50v7j8mezs89tz96'

export const SCROLL_PAIR: IndexerPair = {
  pair_address: SCROLL_PAIR_ADDRESS,
  asset_0: { symbol: 'LUNC', contract_addr: null, denom: 'uluna', decimals: 6 },
  asset_1: { symbol: 'USTC', contract_addr: null, denom: 'uusd', decimals: 6 },
  lp_token: null,
  fee_bps: 30,
  is_active: true,
  liquidity_usd: '10000',
}

const pairInfo = {
  contract_addr: SCROLL_PAIR_ADDRESS,
  liquidity_token: SCROLL_PAIR_ADDRESS,
  asset_infos: [{ native_token: { denom: 'uluna' } }, { native_token: { denom: 'uusd' } }],
}

/** Stable history with enough bars to pan well away from the live edge. */
export function makeScrollCandles(count = 120): IndexerCandle[] {
  return Array.from({ length: count }, (_, index) => {
    const price = 1 + index / 100
    return {
      open_time: new Date(Date.UTC(2026, 9, 1) + index * 3_600_000).toISOString(),
      open: String(price),
      high: String(price + 0.1),
      low: String(price - 0.1),
      close: String(price + 0.02),
      open_human: String(price),
      high_human: String(price + 0.1),
      low_human: String(price - 0.1),
      close_human: String(price + 0.02),
      volume_base: '1000000',
      volume_quote: '2000000',
      trade_count: 1,
    }
  })
}

/** Browser layout evidence uses deterministic market transport, real React and real canvas. */
export async function installScrollInvariantMarket(
  page: Page,
  options: { candles?: () => IndexerCandle[]; pairs?: IndexerPair[] } = {}
): Promise<void> {
  const initialCandles = makeScrollCandles()
  const pairs = options.pairs ?? [SCROLL_PAIR]
  const factoryPairs = pairs.map((pair) => ({
    ...pairInfo,
    contract_addr: pair.pair_address,
    liquidity_token: pair.pair_address,
  }))
  await page.route('**/cosmos/base/tendermint/v1beta1/node_info', (route) =>
    route.fulfill({ json: { default_node_info: { network: 'localterra' } } })
  )
  await page.route('**/api/v1/**', async (route) => {
    const path = new URL(route.request().url()).pathname
    let json: unknown = []
    if (path === '/api/v1/pairs') {
      json = { items: pairs, total: pairs.length, limit: 100, offset: 0 }
    } else if (pairs.some((pair) => path === `/api/v1/pairs/${pair.pair_address}`)) {
      json = pairs.find((pair) => path === `/api/v1/pairs/${pair.pair_address}`)
    } else if (path.endsWith('/candles')) {
      json = options.candles?.() ?? initialCandles
    } else if (path.endsWith('/stats')) {
      json = {
        volume_base: '120000000',
        volume_quote: '240000000',
        volume_usd: '240',
        trade_count: 120,
        high: '2.3',
        low: '0.9',
        open_price: '1',
        close_price: '2.21',
        price_change_pct: 121,
        high_usd: '2.3',
        low_usd: '0.9',
        open_price_usd: '1',
        close_price_usd: '2.21',
      }
    } else if (path.includes('/oracle/price/')) {
      json = { price_usd: '0.02', source: 'fixture', updated_at: '2026-10-04T00:00:00Z' }
    } else if (path.endsWith('/orderbook') || path.endsWith('/orderbook/head')) {
      json = { bids: [], asks: [] }
    }
    await route.fulfill({ json })
  })

  await page.route('**/cosmwasm/wasm/v1/contract/**', async (route) => {
    const url = new URL(route.request().url())
    const smart = url.pathname.match(/\/smart\/([^/]+)$/)
    if (!smart) {
      await route.fulfill({ json: { contract_info: { code_id: '1' } } })
      return
    }
    const query = JSON.parse(Buffer.from(decodeURIComponent(smart[1]!), 'base64').toString())
    let data: unknown = {}
    if (query.pairs) data = { pairs: factoryPairs }
    else if (query.pair) data = pairInfo
    else if (query.pool)
      data = {
        assets: pairInfo.asset_infos.map((info) => ({ info, amount: '1000000000000' })),
        total_share: '1000000000000',
      }
    else if (query.get_asset_code_ids) data = { code_ids: [0, 0] }
    else if (query.is_paused) data = { paused: false }
    else if (query.observe)
      data = {
        price_a_cumulatives: ['86400000000000000000000', '86100000000000000000000', '82800000000000000000000', '0'],
      }
    else if (query.oracle_info)
      data = {
        observations_stored: 120,
        observation_cardinality: 120,
        oldest_observation_timestamp: 0,
        newest_observation_timestamp: 86400,
      }
    else if (query.blacklist_check)
      data = {
        blocked: false,
        wallet_blacklisted: false,
        blacklisted_tokens: [],
        pair_blacklisted: false,
        blacklisted_pairs: [],
      }
    else if (query.is_code_id_whitelisted) data = { whitelisted: true, code_id: 1 }
    else if (query.balance) data = { balance: '0' }
    await route.fulfill({ json: { data } })
  })
}
