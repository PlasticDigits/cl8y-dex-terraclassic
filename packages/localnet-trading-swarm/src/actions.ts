import type { MnemonicWallet } from '@goblinhunt/cosmes/wallet'
import { executeWasm, executeWasmMulti } from './broadcast.js'
import { findRoute } from './graph.js'
import {
  MIN_SWAP_OR_ESCROW_AMOUNT,
  pickScaledProvideAmounts,
  poolReservesOk,
  tokenAddrsForPair,
} from './liquidityGuards.js'
import { queryWasmSmart } from './lcd.js'
import type { ActionKind } from './profiles.js'
import type { AssetInfo, HybridSwapParams, PairInfo, PoolResponse, SwapOperation } from './types.js'
import { assetInfoLabel, tokenAssetInfo } from './types.js'

export interface ActionContext {
  lcdBase: string
  router: string
  pairs: PairInfo[]
  gasPriceUluna: string
  dryRun: boolean
}

interface SimulationResponse {
  return_amount: string
}

function b64(obj: unknown): string {
  return Buffer.from(JSON.stringify(obj), 'utf8').toString('base64')
}

function serializeTerraSwap(ts: SwapOperation['terra_swap']) {
  const out: Record<string, unknown> = {
    offer_asset_info: ts.offer_asset_info,
    ask_asset_info: ts.ask_asset_info,
  }
  if (ts.hybrid) {
    out.hybrid = {
      pool_input: ts.hybrid.pool_input,
      book_input: ts.hybrid.book_input,
      max_maker_fills: ts.hybrid.max_maker_fills,
      book_start_hint: ts.hybrid.book_start_hint ?? undefined,
    }
  }
  return out
}

async function poolForOfferToken(
  lcd: string,
  pairs: PairInfo[],
  token: string,
  tries = 15
): Promise<PoolResponse | null> {
  const cands = pairs.filter((p) => p.asset_infos.some((ai) => assetInfoLabel(ai) === token))
  const shuffled = [...cands].sort(() => Math.random() - 0.5)
  for (const p of shuffled.slice(0, tries)) {
    try {
      const pool = await queryWasmSmart<PoolResponse>(lcd, p.contract_addr, { pool: {} })
      if (poolReservesOk(pool)) return pool
    } catch {
      /* skip */
    }
  }
  return null
}

async function randomLiquidPair(
  lcd: string,
  pairs: PairInfo[],
  tries = 20
): Promise<{ pair: PairInfo; pool: PoolResponse } | null> {
  const shuffled = [...pairs].sort(() => Math.random() - 0.5)
  for (const p of shuffled.slice(0, tries)) {
    try {
      const pool = await queryWasmSmart<PoolResponse>(lcd, p.contract_addr, { pool: {} })
      if (poolReservesOk(pool)) return { pair: p, pool }
    } catch {
      /* next */
    }
  }
  return null
}

function randomCw20PairEndpoints(pairs: PairInfo[]): { from: string; to: string } | null {
  const tokens = new Set<string>()
  for (const p of pairs) {
    for (const ai of p.asset_infos) {
      const x = assetInfoLabel(ai)
      if (x.startsWith('terra1')) tokens.add(x)
    }
  }
  const arr = [...tokens]
  if (arr.length < 2) return null
  const from = arr[Math.floor(Math.random() * arr.length)]!
  let to = arr[Math.floor(Math.random() * arr.length)]!
  let guard = 0
  while (to === from && guard++ < 8) {
    to = arr[Math.floor(Math.random() * arr.length)]!
  }
  if (to === from) return null
  return { from, to }
}

function pickOfferAmount(pool: PoolResponse, offerInfo: AssetInfo): string {
  const side = pool.assets.find((a) => JSON.stringify(a.info) === JSON.stringify(offerInfo))
  const reserve = side ? BigInt(side.amount) : 0n
  const cap = (reserve * 5n) / 10_000n
  const lo = MIN_SWAP_OR_ESCROW_AMOUNT * 10n
  const use = cap > lo ? cap : lo
  if (use < MIN_SWAP_OR_ESCROW_AMOUNT) return '0'
  return use.toString()
}

export async function runAction(
  kind: ActionKind,
  wallet: MnemonicWallet,
  ctx: ActionContext
): Promise<{ action: ActionKind; txHash?: string; dryRun?: boolean; note?: string }> {
  const { lcdBase, router, pairs, gasPriceUluna, dryRun } = ctx

  if (pairs.length === 0) {
    return { action: kind, note: 'no_pairs' }
  }

  if (dryRun) {
    return { action: kind, dryRun: true, note: 'skipped_broadcast' }
  }

  switch (kind) {
    case 'router_multihop': {
      let route: SwapOperation[] | null = null
      let from = ''
      for (let t = 0; t < 25 && !route; t++) {
        const e = randomCw20PairEndpoints(pairs)
        if (!e) break
        const r = findRoute(pairs, e.from, e.to)
        if (r && r.length >= 2) {
          route = r
          from = e.from
          break
        }
      }
      if (!route) {
        const e = randomCw20PairEndpoints(pairs)
        if (!e) return { action: kind, note: 'no_route' }
        const r = findRoute(pairs, e.from, e.to)
        if (!r) return { action: kind, note: 'no_route' }
        route = r
        from = e.from
      }
      const pool = await poolForOfferToken(lcdBase, pairs, from)
      if (!pool) return { action: kind, note: 'no_liquid_pair' }
      const offerInfo = tokenAssetInfo(from)
      const offerAmount = pickOfferAmount(pool, offerInfo)
      if (offerAmount === '0') return { action: kind, note: 'offer_too_small' }
      const inner = {
        execute_swap_operations: {
          operations: route.map((op) => ({ terra_swap: serializeTerraSwap(op.terra_swap) })),
          max_spread: '0.5',
          minimum_receive: undefined,
          to: undefined,
          deadline: undefined,
        },
      }
      const txHash = await executeWasm(
        wallet,
        from,
        {
          send: {
            contract: router,
            amount: offerAmount,
            msg: b64(inner),
          },
        },
        [],
        gasPriceUluna
      )
      return { action: kind, txHash }
    }
    case 'pair_swap': {
      const liq = await randomLiquidPair(lcdBase, pairs)
      if (!liq) return { action: kind, note: 'no_liquid_pair' }
      const { pair, pool } = liq
      const i = Math.random() < 0.5 ? 0 : 1
      const offerInfo = pair.asset_infos[i]!
      const offerToken = assetInfoLabel(offerInfo)
      if (!offerToken.startsWith('terra1')) return { action: kind, note: 'native_offer_skip' }
      const amount = pickOfferAmount(pool, offerInfo)
      if (amount === '0') return { action: kind, note: 'offer_too_small' }
      const swapInner = {
        swap: {
          belief_price: undefined,
          max_spread: '0.5',
          to: undefined,
          deadline: undefined,
          trader: undefined,
          hybrid: undefined,
        },
      }
      const txHash = await executeWasm(
        wallet,
        offerToken,
        {
          send: {
            contract: pair.contract_addr,
            amount,
            msg: b64(swapInner),
          },
        },
        [],
        gasPriceUluna
      )
      return { action: kind, txHash }
    }
    case 'hybrid_swap': {
      const liq = await randomLiquidPair(lcdBase, pairs)
      if (!liq) return { action: kind, note: 'no_liquid_pair' }
      const { pair, pool } = liq
      const idx = Math.random() < 0.5 ? 0 : 1
      const offerInfo = pair.asset_infos[idx]!
      const offerToken = assetInfoLabel(offerInfo)
      if (!offerToken.startsWith('terra1')) return { action: kind, note: 'native_offer_skip' }
      const total = pickOfferAmount(pool, offerInfo)
      if (total === '0') return { action: kind, note: 'offer_too_small' }
      const tot = BigInt(total)
      const poolLeg = (tot * 6n) / 10n
      const bookLeg = tot - poolLeg
      if (bookLeg < MIN_SWAP_OR_ESCROW_AMOUNT || poolLeg < MIN_SWAP_OR_ESCROW_AMOUNT) {
        return { action: kind, note: 'hybrid_split_too_small' }
      }
      const hybrid: HybridSwapParams = {
        pool_input: poolLeg.toString(),
        book_input: bookLeg.toString(),
        max_maker_fills: 8,
        book_start_hint: null,
      }
      const swapInner = {
        swap: {
          belief_price: undefined,
          max_spread: '0.5',
          to: undefined,
          deadline: undefined,
          trader: undefined,
          hybrid: {
            pool_input: hybrid.pool_input,
            book_input: hybrid.book_input,
            max_maker_fills: hybrid.max_maker_fills,
            book_start_hint: hybrid.book_start_hint ?? undefined,
          },
        },
      }
      const txHash = await executeWasm(
        wallet,
        offerToken,
        {
          send: {
            contract: pair.contract_addr,
            amount: total.toString(),
            msg: b64(swapInner),
          },
        },
        [],
        gasPriceUluna
      )
      return { action: kind, txHash }
    }
    case 'limit_order': {
      const liq = await randomLiquidPair(lcdBase, pairs)
      if (!liq) return { action: kind, note: 'no_liquid_pair' }
      const { pair, pool } = liq
      const askSide = Math.random() < 0.5
      const escrowInfo = askSide ? pair.asset_infos[0]! : pair.asset_infos[1]!
      const escrowToken = assetInfoLabel(escrowInfo)
      if (!escrowToken.startsWith('terra1')) return { action: kind, note: 'native_escrow_skip' }
      const amount = pickOfferAmount(pool, escrowInfo)
      if (amount === '0') return { action: kind, note: 'amount_too_small' }
      const sim = await queryWasmSmart<SimulationResponse>(lcdBase, pair.contract_addr, {
        simulation: { offer_asset: { info: escrowInfo, amount } },
      })
      const ret = BigInt(sim.return_amount ?? '1')
      const priceNum = Number(amount) / Number(ret > 0n ? ret : 1n)
      const price = (Number.isFinite(priceNum) && priceNum > 0 ? priceNum : 1).toFixed(6)
      const inner = {
        place_limit_order: {
          side: askSide ? 'ask' : 'bid',
          price,
          hint_after_order_id: null,
          max_adjust_steps: 32,
          expires_at: Math.floor(Date.now() / 1000) + 86_400,
        },
      }
      const txHash = await executeWasm(
        wallet,
        escrowToken,
        {
          send: {
            contract: pair.contract_addr,
            amount,
            msg: b64(inner),
          },
        },
        [],
        gasPriceUluna
      )
      return { action: kind, txHash }
    }
    case 'add_liquidity': {
      const liq = await randomLiquidPair(lcdBase, pairs)
      if (!liq) return { action: kind, note: 'no_liquid_pair' }
      const scaled = pickScaledProvideAmounts(liq.pool, 3000n)
      if (!scaled) return { action: kind, note: 'add_too_small' }
      const [t0, t1] = tokenAddrsForPair(liq.pool)
      const txHash = await executeWasmMulti(
        wallet,
        [
          { contract: t0, msg: { increase_allowance: { spender: liq.pair.contract_addr, amount: scaled.amountA } } },
          { contract: t1, msg: { increase_allowance: { spender: liq.pair.contract_addr, amount: scaled.amountB } } },
          {
            contract: liq.pair.contract_addr,
            msg: {
              provide_liquidity: {
                assets: [
                  { info: tokenAssetInfo(t0), amount: scaled.amountA },
                  { info: tokenAssetInfo(t1), amount: scaled.amountB },
                ],
              },
            },
          },
        ],
        gasPriceUluna
      )
      return { action: kind, txHash }
    }
    case 'remove_liquidity': {
      const liq = await randomLiquidPair(lcdBase, pairs)
      if (!liq) return { action: kind, note: 'no_liquid_pair' }
      const lp = liq.pair.liquidity_token
      const bal = await queryWasmSmart<{ balance: string }>(lcdBase, lp, {
        balance: { address: wallet.address },
      })
      const b = BigInt(bal.balance ?? '0')
      if (b < 10_000n) return { action: kind, note: 'no_lp_balance' }
      const burn = (b * 3n) / 100n
      if (burn < 1000n) return { action: kind, note: 'burn_too_small' }
      const inner = { withdraw_liquidity: { min_assets: undefined } }
      const txHash = await executeWasm(
        wallet,
        lp,
        {
          send: {
            contract: liq.pair.contract_addr,
            amount: burn.toString(),
            msg: b64(inner),
          },
        },
        [],
        gasPriceUluna
      )
      return { action: kind, txHash }
    }
  }
}
