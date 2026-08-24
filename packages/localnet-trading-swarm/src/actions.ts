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
import { filterGemPairs, filterTaxPairs, findTaxInclusiveRoute, randomCw20PairEndpoints } from './pairPick.js'
import { isTaxToken } from './taxDetect.js'
import { DEFAULT_SELL_BPS } from './taxDetect.js'
import { balanceCoversDebit, requiredWalletDebit, taxLogFields, type TaxLogFields } from './taxPreview.js'
import { pairDirectSwapHook, routerExecuteSwapOperations, routerHopSwapPreviewHook } from './taxHooks.js'
import { queryCw20Balance, queryTaxPreview } from './taxQuery.js'

export interface ActionContext {
  lcdBase: string
  router: string
  pairs: PairInfo[]
  gasPriceUluna: string
  dryRun: boolean
  taxTokens?: Set<string>
  taxMode?: boolean
  sellBps?: number
}

export interface ActionResult {
  action: ActionKind
  txHash?: string
  dryRun?: boolean
  note?: string
  tax_debit?: string
  tax_credit?: string
  bps?: number
  path?: 'pair' | 'router'
}

interface HybridSimulationResponse {
  return_amount: string
}

function b64(obj: unknown): string {
  return Buffer.from(JSON.stringify(obj), 'utf8').toString('base64')
}

function taxSet(ctx: ActionContext): Set<string> {
  return ctx.taxTokens ?? new Set()
}

function workingPairs(ctx: ActionContext): PairInfo[] {
  const taxes = taxSet(ctx)
  return ctx.taxMode ? filterTaxPairs(ctx.pairs, taxes) : filterGemPairs(ctx.pairs, taxes)
}

function routeGraphPairs(ctx: ActionContext): PairInfo[] {
  return ctx.taxMode ? ctx.pairs : workingPairs(ctx)
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

function pickOfferAmount(pool: PoolResponse, offerInfo: AssetInfo): string {
  const side = pool.assets.find((a) => JSON.stringify(a.info) === JSON.stringify(offerInfo))
  const reserve = side ? BigInt(side.amount) : 0n
  const cap = (reserve * 5n) / 10_000n
  const lo = MIN_SWAP_OR_ESCROW_AMOUNT * 10n
  const use = cap > lo ? cap : lo
  if (use < MIN_SWAP_OR_ESCROW_AMOUNT) return '0'
  return use.toString()
}

function pairForHop(pairs: PairInfo[], offer: string, ask: string): PairInfo | undefined {
  return pairs.find((p) => {
    const a = assetInfoLabel(p.asset_infos[0])
    const b = assetInfoLabel(p.asset_infos[1])
    return (a === offer && b === ask) || (a === ask && b === offer)
  })
}

async function gateTaxSell(input: {
  ctx: ActionContext
  wallet: string
  offerToken: string
  amount: string
  path: 'pair' | 'router'
  pairAddr?: string
}): Promise<{ ok: true; logs: TaxLogFields } | { ok: false; note: string; logs?: TaxLogFields }> {
  const taxes = taxSet(input.ctx)
  if (!isTaxToken(input.offerToken, taxes)) {
    return { ok: true, logs: { tax_debit: input.amount, tax_credit: input.amount, bps: 0, path: input.path } }
  }
  const sellBps = input.ctx.sellBps ?? DEFAULT_SELL_BPS
  const amt = BigInt(input.amount)
  let preview = null
  if (input.path === 'pair' && input.pairAddr) {
    preview = await queryTaxPreview({
      lcdBase: input.ctx.lcdBase,
      token: input.offerToken,
      from: input.wallet,
      to: input.pairAddr,
      amount: input.amount,
      sendMsgB64: b64(pairDirectSwapHook()),
    })
  } else if (input.path === 'router' && input.pairAddr) {
    preview = await queryTaxPreview({
      lcdBase: input.ctx.lcdBase,
      token: input.offerToken,
      from: input.ctx.router,
      to: input.pairAddr,
      amount: input.amount,
      sendMsgB64: b64(routerHopSwapPreviewHook(input.wallet)),
    })
  }
  const required = requiredWalletDebit(preview, amt, input.path, sellBps)
  const logs = taxLogFields(preview, amt, input.path, sellBps)
  const bal = await queryCw20Balance(input.ctx.lcdBase, input.offerToken, input.wallet)
  if (!balanceCoversDebit(bal, required)) {
    return { ok: false, note: 'tax_balance_short', logs }
  }
  return { ok: true, logs }
}

function withTaxLogs(base: ActionResult, logs?: TaxLogFields): ActionResult {
  if (!logs) return base
  return { ...base, ...logs }
}

export async function runAction(
  kind: ActionKind,
  wallet: MnemonicWallet,
  ctx: ActionContext
): Promise<ActionResult> {
  const { lcdBase, router, gasPriceUluna, dryRun } = ctx
  const pairs = workingPairs(ctx)
  const graphPairs = routeGraphPairs(ctx)
  const taxes = taxSet(ctx)

  if (ctx.pairs.length === 0) {
    return { action: kind, note: 'no_pairs' }
  }

  if (dryRun) {
    const taxVisible = filterTaxPairs(ctx.pairs, taxes).length > 0
    const note = ctx.taxMode
      ? taxVisible
        ? 'tax_pair_visible'
        : 'tax_pair_missing'
      : 'skipped_broadcast'
    return { action: kind, dryRun: true, note }
  }

  if (ctx.taxMode && kind === 'hybrid_swap') {
    return { action: kind, note: 'tax_hybrid_skip' }
  }

  if (pairs.length === 0) {
    return { action: kind, note: ctx.taxMode ? 'no_tax_pair' : 'no_gem_pairs' }
  }

  switch (kind) {
    case 'router_multihop': {
      let route: SwapOperation[] | null = null
      let from = ''
      if (ctx.taxMode) {
        const hit = findTaxInclusiveRoute(graphPairs, taxes, Math.random() < 0.6)
        if (!hit) return { action: kind, note: 'no_tax_route' }
        route = hit.route
        from = hit.from
      } else {
        for (let t = 0; t < 25 && !route; t++) {
          const e = randomCw20PairEndpoints(pairs, taxes)
          if (!e) break
          const r = findRoute(graphPairs, e.from, e.to)
          if (r && r.length >= 2) {
            route = r
            from = e.from
            break
          }
        }
        if (!route) {
          const e = randomCw20PairEndpoints(pairs, taxes)
          if (!e) return { action: kind, note: 'no_route' }
          const r = findRoute(graphPairs, e.from, e.to)
          if (!r) return { action: kind, note: 'no_route' }
          route = r
          from = e.from
        }
      }
      const pool = await poolForOfferToken(lcdBase, graphPairs, from)
      if (!pool) return { action: kind, note: 'no_liquid_pair' }
      const offerInfo = tokenAssetInfo(from)
      const offerAmount = pickOfferAmount(pool, offerInfo)
      if (offerAmount === '0') return { action: kind, note: 'offer_too_small' }
      const firstAsk = assetInfoLabel(route[0]!.terra_swap.ask_asset_info)
      const firstPair = pairForHop(graphPairs, from, firstAsk)
      const gated = await gateTaxSell({
        ctx,
        wallet: wallet.address,
        offerToken: from,
        amount: offerAmount,
        path: 'router',
        pairAddr: firstPair?.contract_addr,
      })
      if (!gated.ok) return withTaxLogs({ action: kind, note: gated.note }, gated.logs)
      const inner = routerExecuteSwapOperations(
        route.map((op) => ({ terra_swap: serializeTerraSwap(op.terra_swap) }))
      )
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
      return withTaxLogs({ action: kind, txHash }, gated.logs)
    }
    case 'pair_swap': {
      const liq = await randomLiquidPair(lcdBase, pairs)
      if (!liq) return { action: kind, note: 'no_liquid_pair' }
      const { pair, pool } = liq
      const i = Math.random() < 0.5 ? 0 : 1
      const offerInfo = pair.asset_infos[i]!
      const offerToken = assetInfoLabel(offerInfo)
      if (!offerToken.startsWith('terra1')) return { action: kind, note: 'native_offer_skip' }
      if (!ctx.taxMode && isTaxToken(offerToken, taxes)) {
        return { action: kind, note: 'gem_tax_exclude' }
      }
      const amount = pickOfferAmount(pool, offerInfo)
      if (amount === '0') return { action: kind, note: 'offer_too_small' }
      const swapInner = pairDirectSwapHook()
      const gated = await gateTaxSell({
        ctx,
        wallet: wallet.address,
        offerToken,
        amount,
        path: 'pair',
        pairAddr: pair.contract_addr,
      })
      if (!gated.ok) return withTaxLogs({ action: kind, note: gated.note }, gated.logs)
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
      return withTaxLogs({ action: kind, txHash }, gated.logs)
    }
    case 'hybrid_swap': {
      const liq = await randomLiquidPair(lcdBase, pairs)
      if (!liq) return { action: kind, note: 'no_liquid_pair' }
      const { pair, pool } = liq
      const idx = Math.random() < 0.5 ? 0 : 1
      const offerInfo = pair.asset_infos[idx]!
      const offerToken = assetInfoLabel(offerInfo)
      if (!offerToken.startsWith('terra1')) return { action: kind, note: 'native_offer_skip' }
      if (isTaxToken(offerToken, taxes)) {
        return { action: kind, note: 'tax_hybrid_skip' }
      }
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
          max_spread: '1',
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
      const sim = await queryWasmSmart<HybridSimulationResponse>(lcdBase, pair.contract_addr, {
        hybrid_simulation: {
          offer_asset: { info: escrowInfo, amount },
          hybrid: {
            pool_input: amount,
            book_input: '0',
            max_maker_fills: 1,
            book_start_hint: undefined,
          },
        },
      })
      const ret = BigInt(sim.return_amount ?? '1')
      const priceNum = Number(amount) / Number(ret > 0n ? ret : 1n)
      const price = (Number.isFinite(priceNum) && priceNum > 0 ? priceNum : 1).toFixed(6)
      const inner = {
        place_limit_order_batch: {
          side: askSide ? 'ask' : 'bid',
          orders: [
            {
              price,
              amount,
              max_adjust_steps: 32,
              expires_at: Math.floor(Date.now() / 1000) + 86_400,
            },
          ],
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
      return { action: kind, txHash, path: 'pair', bps: 0, tax_debit: amount, tax_credit: amount }
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
      return { action: kind, txHash, path: 'pair', bps: 0 }
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
