/**
 * peggedassets-server copy: `src/adapters/peggedAssets/ust1/index.ts`
 *
 * UST1 unstablecoin on Terra Classic. Folder name is `ust1` (no CoinGecko id).
 * Circulating = CW20 `token_info.total_supply` / 1e6. Returns `{ peggedUSD }`.
 * Mechanism (Discord metadata): crypto-backed via ust1-window / vFDUSD.
 * Do not hardcode $1. Do not list USTR here.
 *
 * Submit upstream to https://github.com/DefiLlama/peggedassets-server
 * Test there: `npx ts-node --transpile-only test.ts ust1 peggedUSD`
 *
 * LCD query is self-contained so this file does not depend on indexer USD.
 */

const UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
const UST1_DECIMALS = 6
const TERRA_LCD = 'https://terra-classic-lcd.publicnode.com'

function circulatingFromTokenInfo(tokenInfo: { total_supply?: string }, decimals = UST1_DECIMALS) {
  const raw = tokenInfo && tokenInfo.total_supply
  if (raw == null || raw === '') return null
  const n = Number(raw)
  if (!Number.isFinite(n) || n < 0) return null
  return n / 10 ** decimals
}

async function queryTokenInfo(contract: string) {
  const query = Buffer.from(JSON.stringify({ token_info: {} })).toString('base64')
  const url = `${TERRA_LCD}/cosmwasm/wasm/v1/contract/${contract}/smart/${query}`
  const res = await fetch(url)
  if (!res.ok) {
    throw new Error(`UST1 token_info LCD ${res.status}`)
  }
  const json = (await res.json()) as { data?: { total_supply?: string } }
  return json.data || {}
}

async function terraMinted() {
  const info = await queryTokenInfo(UST1)
  const circulating = circulatingFromTokenInfo(info)
  if (circulating == null) {
    throw new Error('UST1 token_info.total_supply missing or invalid')
  }
  return { peggedUSD: circulating }
}

const adapter = {
  terra: {
    minted: terraMinted,
    unreleased: async () => ({}),
  },
}

export default adapter
