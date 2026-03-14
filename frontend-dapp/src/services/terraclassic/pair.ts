import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import type {
  Asset,
  AssetInfo,
  PairInfo,
  PoolResponse,
  SimulationResponse,
  ReverseSimulationResponse,
} from '@/types'
import { tokenAssetInfo } from '@/types'

export async function getPairInfo(pairAddress: string): Promise<PairInfo> {
  return queryContract<PairInfo>(pairAddress, { pair: {} })
}

export async function getPool(pairAddress: string): Promise<PoolResponse> {
  return queryContract<PoolResponse>(pairAddress, { pool: {} })
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

export async function swap(
  walletAddress: string,
  tokenAddress: string,
  pairAddress: string,
  amount: string,
  beliefPrice?: string,
  maxSpread?: string,
  to?: string
): Promise<string> {
  const swapMsg = btoa(
    JSON.stringify({
      swap: {
        belief_price: beliefPrice,
        max_spread: maxSpread,
        to,
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
    } catch { /* best effort cleanup */ }
    try {
      await executeTerraContract(walletAddress, tokenB, {
        decrease_allowance: { spender: pairAddress, amount: amountB },
      })
    } catch { /* best effort cleanup */ }
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
