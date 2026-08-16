import { queryContract } from './queries'
import { poolOnlyHybridParams, poolOnlyHybridTemplate } from './poolOnlyHybrid'
import { executeCw20AllowanceThen, executeTerraContract, executeTerraContractMulti } from './transactions'
import type {
  Asset,
  AssetInfo,
  HybridReverseSimulationResponse,
  HybridSimulationResponse,
  HybridSwapParams,
  PairInfo,
  PairOrderStatusKind,
  PairOrderStatusResponse,
  PairPausedResponse,
  PoolResponse,
} from '@/types'
import { tokenAssetInfo } from '@/types'

/** Optional wallet for CL8Y fee-tier parity on hybrid LCD quotes (GitLab #245). */
export interface QuoteTraderOptions {
  trader?: string
  sender?: string
}

export async function getPairInfo(pairAddress: string): Promise<PairInfo> {
  return queryContract<PairInfo>(pairAddress, { pair: {} })
}

export async function getPool(pairAddress: string): Promise<PoolResponse> {
  return queryContract<PoolResponse>(pairAddress, { pool: {} })
}

export async function getPairPaused(pairAddress: string): Promise<PairPausedResponse> {
  return queryContract<PairPausedResponse>(pairAddress, { is_paused: {} })
}

/** Parse a successful LCD `OrderStatus` decode. Failures must stay errors — never coerce to `unknown` (L21). */
export function parsePairOrderStatus(
  raw: PairOrderStatusResponse | string | null | undefined
): PairOrderStatusKind | undefined {
  const s = (typeof raw === 'string' ? raw : raw?.status)?.trim().toLowerCase()
  if (s === 'active' || s === 'parked_refund' || s === 'unknown') return s
  return undefined
}

/**
 * Typed custody lookup (`QueryMsg::OrderStatus`). `order_id == 0` is invalid on-chain.
 * Callers must not treat a thrown error as `Unknown` (GitLab #505 / #530).
 */
export async function queryOrderStatus(pairAddress: string, orderId: number): Promise<PairOrderStatusResponse> {
  if (!Number.isFinite(orderId) || orderId < 1) {
    throw new Error('Invalid order id')
  }
  return queryContract<PairOrderStatusResponse>(pairAddress, {
    order_status: { order_id: orderId },
  })
}

/** Pool-only forward quote via `hybrid_simulation` (GitLab #190). */
export async function simulateSwap(
  pairAddress: string,
  offerAssetInfo: AssetInfo,
  offerAmount: string,
  options?: QuoteTraderOptions
): Promise<HybridSimulationResponse> {
  return simulateHybridSwap(pairAddress, offerAssetInfo, offerAmount, poolOnlyHybridParams(offerAmount), options)
}

export async function simulateHybridSwap(
  pairAddress: string,
  offerAssetInfo: AssetInfo,
  offerAmount: string,
  hybrid: HybridSwapParams,
  options?: QuoteTraderOptions
): Promise<HybridSimulationResponse> {
  const offerAsset: Asset = { info: offerAssetInfo, amount: offerAmount }
  return queryContract<HybridSimulationResponse>(pairAddress, {
    hybrid_simulation: {
      offer_asset: offerAsset,
      hybrid: {
        pool_input: hybrid.pool_input,
        book_input: hybrid.book_input,
        max_maker_fills: hybrid.max_maker_fills,
        book_start_hint: hybrid.book_start_hint ?? undefined,
      },
      trader: options?.trader,
      sender: options?.sender,
    },
  })
}

/** Pool-only reverse quote via `hybrid_reverse_simulation` (GitLab #190). */
export async function reverseSimulateSwap(
  pairAddress: string,
  askAssetInfo: AssetInfo,
  askAmount: string
): Promise<HybridReverseSimulationResponse> {
  const askAsset: Asset = { info: askAssetInfo, amount: askAmount }
  return queryContract<HybridReverseSimulationResponse>(pairAddress, {
    hybrid_reverse_simulation: {
      ask_asset: askAsset,
      hybrid: poolOnlyHybridTemplate(),
    },
  })
}

export interface DirectSwapOptions {
  hybrid?: HybridSwapParams | null
  /** Minimum net ask output when the book leg is present without `belief_price` (GitLab #334). */
  minReturn?: string | null
  deadline?: number | null
  trader?: string | null
}

export async function swap(
  walletAddress: string,
  tokenAddress: string,
  pairAddress: string,
  amount: string,
  beliefPrice?: string,
  maxSpread?: string,
  to?: string,
  options?: DirectSwapOptions
): Promise<string> {
  const hybrid = options?.hybrid
  const swapMsg = btoa(
    JSON.stringify({
      swap: {
        belief_price: beliefPrice,
        max_spread: maxSpread,
        min_return: options?.minReturn ?? undefined,
        to,
        deadline: options?.deadline ?? undefined,
        trader: options?.trader ?? undefined,
        hybrid: hybrid
          ? {
              pool_input: hybrid.pool_input,
              book_input: hybrid.book_input,
              max_maker_fills: hybrid.max_maker_fills,
              book_start_hint: hybrid.book_start_hint ?? undefined,
            }
          : undefined,
      },
    })
  )
  return executeTerraContract(walletAddress, tokenAddress, {
    send: {
      contract: pairAddress,
      amount,
      msg: swapMsg,
    },
  })
}

export interface LimitOrderPlacementItemWire {
  price: string
  amount: string
  max_adjust_steps: number
  expires_at?: number
  hint_after_order_id?: number | null
}

export interface LimitOrderLadderSpecWire {
  side: 'bid' | 'ask'
  start_price: string
  end_price: string
  count: number
  total_amount: string
  distribution: 'equal'
  max_adjust_steps: number
  expires_at?: number
  /** Head-most rung anchor from indexer (#266 / #267). */
  hint_after_order_id?: number
}

export interface LimitOrderConfigResponse {
  max_batch_rungs: number
}

export async function getLimitOrderConfig(pairAddress: string): Promise<LimitOrderConfigResponse> {
  return queryContract<LimitOrderConfigResponse>(pairAddress, { limit_order_config: {} })
}

function encodeLimitBatchHook(side: 'bid' | 'ask', orders: LimitOrderPlacementItemWire[]): string {
  return btoa(
    JSON.stringify({
      place_limit_order_batch: { side, orders },
    })
  )
}

/** Single-rung batch placement (GitLab #206). */
export async function placeLimitOrderBatch(
  walletAddress: string,
  escrowTokenAddress: string,
  pairAddress: string,
  totalAmount: string,
  side: 'bid' | 'ask',
  orders: LimitOrderPlacementItemWire[]
): Promise<string> {
  const msg = encodeLimitBatchHook(side, orders)
  return executeTerraContract(walletAddress, escrowTokenAddress, {
    send: {
      contract: pairAddress,
      amount: totalAmount,
      msg,
    },
  })
}

export async function placeLimitOrderLadder(
  walletAddress: string,
  escrowTokenAddress: string,
  pairAddress: string,
  totalAmount: string,
  ladder: LimitOrderLadderSpecWire
): Promise<string> {
  const msg = btoa(JSON.stringify({ place_limit_order_ladder: { ladder } }))
  return executeTerraContract(walletAddress, escrowTokenAddress, {
    send: {
      contract: pairAddress,
      amount: totalAmount,
      msg,
    },
  })
}

/**
 * Retail limit path: `increase_allowance` then CW20 `send` → batch/ladder hook.
 */
export async function placeLimitOrderBatchWithAllowance(
  walletAddress: string,
  escrowTokenAddress: string,
  pairAddress: string,
  totalAmount: string,
  side: 'bid' | 'ask',
  orders: LimitOrderPlacementItemWire[]
): Promise<string> {
  return executeCw20AllowanceThen(walletAddress, escrowTokenAddress, pairAddress, totalAmount, () =>
    placeLimitOrderBatch(walletAddress, escrowTokenAddress, pairAddress, totalAmount, side, orders)
  )
}

export async function placeLimitOrderLadderWithAllowance(
  walletAddress: string,
  escrowTokenAddress: string,
  pairAddress: string,
  totalAmount: string,
  ladder: LimitOrderLadderSpecWire
): Promise<string> {
  return executeCw20AllowanceThen(walletAddress, escrowTokenAddress, pairAddress, totalAmount, () =>
    placeLimitOrderLadder(walletAddress, escrowTokenAddress, pairAddress, totalAmount, ladder)
  )
}

/** Convenience: one order via batch hook with `orders.len() === 1`. */
export async function placeLimitOrderWithAllowance(
  walletAddress: string,
  escrowTokenAddress: string,
  pairAddress: string,
  amount: string,
  side: 'bid' | 'ask',
  price: string,
  maxAdjustSteps: number,
  expiresAt?: number | null,
  hintAfterOrderId?: number | null
): Promise<string> {
  const item: LimitOrderPlacementItemWire = {
    price,
    amount,
    max_adjust_steps: maxAdjustSteps,
    expires_at: expiresAt ?? undefined,
  }
  if (hintAfterOrderId != null) {
    item.hint_after_order_id = hintAfterOrderId
  }
  return placeLimitOrderBatchWithAllowance(walletAddress, escrowTokenAddress, pairAddress, amount, side, [item])
}

export async function cancelLimitOrder(walletAddress: string, pairAddress: string, orderId: number): Promise<string> {
  return executeTerraContract(walletAddress, pairAddress, {
    cancel_limit_order: { order_id: orderId },
  })
}

/** Owner-only in-place price relink — no maker fee, no CW20 movement (GitLab #247). */
export async function updateLimitOrderPrice(
  walletAddress: string,
  pairAddress: string,
  orderId: number,
  price: string,
  maxAdjustSteps: number,
  hintAfterOrderId?: number | null
): Promise<string> {
  return executeTerraContract(walletAddress, pairAddress, {
    update_limit_order_price: {
      order_id: orderId,
      price,
      hint_after_order_id: hintAfterOrderId ?? null,
      max_adjust_steps: maxAdjustSteps,
    },
  })
}

/** Batch cancel resting limits (GitLab #246). All ids must belong to `walletAddress`; whole tx reverts on any failure. */
export async function cancelLimitOrders(
  walletAddress: string,
  pairAddress: string,
  orderIds: number[]
): Promise<string> {
  return executeTerraContract(walletAddress, pairAddress, {
    cancel_limit_orders: { order_ids: orderIds },
  })
}

/** Owner-only refund after indexer `lifecycle_status: parked_expired` (pair `ClaimExpiredLimitOrder`). Blocked while pair is paused (L6 / GitLab #120). */
export async function claimExpiredLimitOrder(
  walletAddress: string,
  pairAddress: string,
  orderId: number
): Promise<string> {
  return executeTerraContract(walletAddress, pairAddress, {
    claim_expired_limit_order: { order_id: orderId },
  })
}

/** Batch claim parked-expiry refund rows (GitLab #246). */
export async function claimExpiredLimitOrders(
  walletAddress: string,
  pairAddress: string,
  orderIds: number[]
): Promise<string> {
  return executeTerraContract(walletAddress, pairAddress, {
    claim_expired_limit_orders: { order_ids: orderIds },
  })
}

export async function provideLiquidity(
  walletAddress: string,
  pairAddress: string,
  tokenA: string,
  tokenB: string,
  amountA: string,
  amountB: string
): Promise<string> {
  await executeTerraContract(walletAddress, tokenA, {
    increase_allowance: { spender: pairAddress, amount: amountA },
  })

  await executeTerraContract(walletAddress, tokenB, {
    increase_allowance: { spender: pairAddress, amount: amountB },
  })

  const assets: [Asset, Asset] = [
    { info: tokenAssetInfo(tokenA), amount: amountA },
    { info: tokenAssetInfo(tokenB), amount: amountB },
  ]

  try {
    const txHash = await executeTerraContract(walletAddress, pairAddress, {
      provide_liquidity: { assets },
    })
    return txHash
  } catch (error) {
    try {
      /** Single tx (one prompt / one fee) vs two sequential `decrease_allowance` broadcasts ([GitLab #147](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/147)). */
      await executeTerraContractMulti(walletAddress, [
        {
          contract: tokenA,
          msg: { decrease_allowance: { spender: pairAddress, amount: amountA } },
        },
        {
          contract: tokenB,
          msg: { decrease_allowance: { spender: pairAddress, amount: amountB } },
        },
      ])
    } catch {
      /* best effort cleanup */
    }
    throw error
  }
}

export async function withdrawLiquidity(
  walletAddress: string,
  lpTokenAddress: string,
  pairAddress: string,
  amount: string,
  minAssets?: [string, string]
): Promise<string> {
  const withdrawMsg = btoa(JSON.stringify({ withdraw_liquidity: { min_assets: minAssets } }))
  return executeTerraContract(walletAddress, lpTokenAddress, {
    send: {
      contract: pairAddress,
      amount,
      msg: withdrawMsg,
    },
  })
}
