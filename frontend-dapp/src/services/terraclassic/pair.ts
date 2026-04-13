import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import type {
  Asset,
  AssetInfo,
  HybridSwapParams,
  PairInfo,
  PairPausedResponse,
  PoolResponse,
  ReverseSimulationResponse,
  SimulationResponse,
} from '@/types'
import { tokenAssetInfo } from '@/types'

export async function getPairInfo(pairAddress: string): Promise<PairInfo> {
  return queryContract<PairInfo>(pairAddress, { pair: {} })
}

export async function getPool(pairAddress: string): Promise<PoolResponse> {
  return queryContract<PoolResponse>(pairAddress, { pool: {} })
}

export async function getPairPaused(pairAddress: string): Promise<PairPausedResponse> {
  return queryContract<PairPausedResponse>(pairAddress, { is_paused: {} })
}

export async function simulateSwap(
  pairAddress: string,
  offerAssetInfo: AssetInfo,
  offerAmount: string
): Promise<SimulationResponse> {
  const offerAsset: Asset = { info: offerAssetInfo, amount: offerAmount }
  return queryContract<SimulationResponse>(pairAddress, {
    simulation: { offer_asset: offerAsset },
  })
}

export async function reverseSimulateSwap(
  pairAddress: string,
  askAssetInfo: AssetInfo,
  askAmount: string
): Promise<ReverseSimulationResponse> {
  const askAsset: Asset = { info: askAssetInfo, amount: askAmount }
  return queryContract<ReverseSimulationResponse>(pairAddress, {
    reverse_simulation: { ask_asset: askAsset },
  })
}

export interface DirectSwapOptions {
  hybrid?: HybridSwapParams | null
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

/** Bid escrows token1; Ask escrows token0 (pair asset ordering). */
export async function placeLimitOrder(
  walletAddress: string,
  escrowTokenAddress: string,
  pairAddress: string,
  amount: string,
  side: 'bid' | 'ask',
  price: string,
  maxAdjustSteps: number,
  expiresAt?: number | null
): Promise<string> {
  const msg = btoa(
    JSON.stringify({
      place_limit_order: {
        side,
        price,
        hint_after_order_id: null,
        max_adjust_steps: maxAdjustSteps,
        expires_at: expiresAt ?? undefined,
      },
    })
  )
  return executeTerraContract(walletAddress, escrowTokenAddress, {
    send: {
      contract: pairAddress,
      amount,
      msg,
    },
  })
}

export async function cancelLimitOrder(walletAddress: string, pairAddress: string, orderId: number): Promise<string> {
  return executeTerraContract(walletAddress, pairAddress, {
    cancel_limit_order: { order_id: orderId },
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
      await executeTerraContract(walletAddress, tokenA, {
        decrease_allowance: { spender: pairAddress, amount: amountA },
      })
    } catch {
      /* best effort cleanup */
    }
    try {
      await executeTerraContract(walletAddress, tokenB, {
        decrease_allowance: { spender: pairAddress, amount: amountB },
      })
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
