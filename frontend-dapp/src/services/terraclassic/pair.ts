import { queryContract } from './queries'
import { executeTerraContract } from './transactions'
import type { PairInfo, FeeConfig, ReservesInfo, SimulateSwapResult } from '@/types'

export async function getPairInfo(pairAddress: string): Promise<PairInfo> {
  const resp = await queryContract<{ pair_info: PairInfo }>(pairAddress, { get_pair_info: {} })
  return resp.pair_info
}

export async function getReserves(pairAddress: string): Promise<ReservesInfo> {
  return queryContract<ReservesInfo>(pairAddress, { get_reserves: {} })
}

export async function getFeeConfig(pairAddress: string): Promise<FeeConfig> {
  return queryContract<FeeConfig>(pairAddress, { get_fee_config: {} })
}

export async function simulateSwap(
  pairAddress: string,
  offerToken: string,
  offerAmount: string
): Promise<SimulateSwapResult> {
  return queryContract<SimulateSwapResult>(pairAddress, {
    simulate_swap: { offer_token: offerToken, offer_amount: offerAmount },
  })
}

export async function swap(
  walletAddress: string,
  tokenAddress: string,
  pairAddress: string,
  amount: string,
  minOutput?: string,
  to?: string
): Promise<string> {
  const swapMsg = btoa(JSON.stringify({
    swap: { min_output: minOutput, to },
  }))
  return executeTerraContract(walletAddress, tokenAddress, {
    send: {
      contract: pairAddress,
      amount,
      msg: swapMsg,
    },
  })
}

export async function addLiquidity(
  walletAddress: string,
  pairAddress: string,
  tokenA: string,
  tokenB: string,
  amountA: string,
  amountB: string,
  minLpTokens?: string
): Promise<string> {
  await executeTerraContract(walletAddress, tokenA, {
    increase_allowance: { spender: pairAddress, amount: amountA },
  })

  await executeTerraContract(walletAddress, tokenB, {
    increase_allowance: { spender: pairAddress, amount: amountB },
  })

  return executeTerraContract(walletAddress, pairAddress, {
    add_liquidity: {
      token_a_amount: amountA,
      token_b_amount: amountB,
      min_lp_tokens: minLpTokens,
    },
  })
}

export async function removeLiquidity(
  walletAddress: string,
  lpTokenAddress: string,
  pairAddress: string,
  amount: string,
  minA?: string,
  minB?: string
): Promise<string> {
  const removeMsg = btoa(JSON.stringify({
    remove_liquidity: { min_a: minA, min_b: minB },
  }))
  return executeTerraContract(walletAddress, lpTokenAddress, {
    send: {
      contract: pairAddress,
      amount,
      msg: removeMsg,
    },
  })
}
