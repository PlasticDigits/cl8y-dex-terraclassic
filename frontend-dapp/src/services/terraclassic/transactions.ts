import { MsgExecuteContract } from '@goblinhunt/cosmes/client'
import type { UnsignedTx } from '@goblinhunt/cosmes/wallet'
import { CosmosTxV1beta1Fee as Fee } from '@goblinhunt/cosmes/protobufs'
import { getConnectedWallet } from './wallet'
import {
  EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP,
  GAS_PRICE_ULUNA,
  SWAP_GAS_BUFFER,
  SWAP_GAS_PER_HOP,
  SWAP_MULTIHOP_GAS_PADDING_PER_HOP,
  WRAP_GAS_LIMIT,
} from '@/utils/constants'
const BASE_GAS_LIMIT = 200000
const SWAP_GAS_LIMIT = 600000
const ADD_LIQUIDITY_GAS_LIMIT = 500000
const REMOVE_LIQUIDITY_GAS_LIMIT = 600000
const CREATE_PAIR_GAS_LIMIT = 800000

function estimateTerraClassicFee(gasLimit: number): Fee {
  const feeAmount = Math.ceil(parseFloat(GAS_PRICE_ULUNA) * gasLimit)

  return new Fee({
    amount: [
      {
        amount: feeAmount.toString(),
        denom: 'uluna',
      },
    ],
    gasLimit: BigInt(gasLimit),
  })
}

function countSwapHops(msg: Record<string, unknown>): number {
  const ops = (msg as { execute_swap_operations?: { operations?: unknown[] } }).execute_swap_operations
  return ops?.operations?.length ?? 1
}

/** Buffered estimate + per-hop padding, floored at min gas per hop (see constants). */
function gasLimitForExecuteSwapOperations(hops: number): number {
  const hopCount = Math.max(hops, 1)
  const scaled = Math.round(SWAP_GAS_PER_HOP * hopCount * SWAP_GAS_BUFFER)
  const padded = scaled + hopCount * SWAP_MULTIHOP_GAS_PADDING_PER_HOP
  const floor = hopCount * EXECUTE_SWAP_OPS_MIN_GAS_PER_HOP
  return Math.max(padded, floor)
}

function getGasLimitForTx(executeMsg: Record<string, unknown>): number {
  if ('wrap_deposit' in executeMsg) {
    return WRAP_GAS_LIMIT
  }
  if ('execute_swap_operations' in executeMsg) {
    return gasLimitForExecuteSwapOperations(countSwapHops(executeMsg))
  } else if ('swap' in executeMsg) {
    return SWAP_GAS_LIMIT
  } else if ('provide_liquidity' in executeMsg) {
    return ADD_LIQUIDITY_GAS_LIMIT
  } else if ('withdraw_liquidity' in executeMsg) {
    return REMOVE_LIQUIDITY_GAS_LIMIT
  } else if ('create_pair' in executeMsg) {
    return CREATE_PAIR_GAS_LIMIT
  } else if ('send' in executeMsg) {
    const sendMsg = executeMsg.send as { msg?: string } | undefined
    if (sendMsg?.msg) {
      try {
        const inner = JSON.parse(atob(sendMsg.msg))
        if ('swap' in inner) return SWAP_GAS_LIMIT
        if ('withdraw_liquidity' in inner) return REMOVE_LIQUIDITY_GAS_LIMIT
        if ('execute_swap_operations' in inner) return gasLimitForExecuteSwapOperations(countSwapHops(inner))
      } catch {
        // fall through to base
      }
    }
    return SWAP_GAS_LIMIT
  } else if ('increase_allowance' in executeMsg) {
    return BASE_GAS_LIMIT
  }
  return BASE_GAS_LIMIT
}

/**
 * Execute a contract on Terra Classic.
 * @param walletAddress - The sender address
 * @param contractAddress - The contract to execute
 * @param executeMsg - The execute message
 * @param coins - Optional coins to send with the transaction
 * @returns Transaction hash
 */
export async function executeTerraContract(
  walletAddress: string,
  contractAddress: string,
  executeMsg: Record<string, unknown>,
  coins?: Array<{ denom: string; amount: string }>
): Promise<string> {
  const wallet = getConnectedWallet()
  if (!wallet) {
    throw new Error('Wallet not connected. Please connect your wallet first.')
  }

  if (wallet.address !== walletAddress) {
    throw new Error('Wallet address mismatch')
  }

  try {
    const msg = new MsgExecuteContract({
      sender: walletAddress,
      contract: contractAddress,
      msg: executeMsg,
      funds: coins && coins.length > 0 ? coins : [],
    })

    const unsignedTx: UnsignedTx = {
      msgs: [msg],
      memo: '',
    }

    const gasLimit = getGasLimitForTx(executeMsg)
    const fee = estimateTerraClassicFee(gasLimit)

    const txHash = await wallet.broadcastTx(unsignedTx, fee)
    const { txResponse } = await wallet.pollTx(txHash)

    if (txResponse.code !== 0) {
      const errorMsg =
        txResponse.rawLog || txResponse.logs?.[0]?.log || `Transaction failed with code ${txResponse.code}`
      throw new Error(`Transaction failed: ${errorMsg}`)
    }

    return txHash
  } catch (error: unknown) {
    console.error('Terra Classic transaction error:', error)
    throw handleTransactionError(error)
  }
}

export async function executeTerraContractMulti(
  walletAddress: string,
  messages: Array<{
    contract: string
    msg: Record<string, unknown>
    coins?: Array<{ denom: string; amount: string }>
  }>
): Promise<string> {
  const wallet = getConnectedWallet()
  if (!wallet) {
    throw new Error('Wallet not connected. Please connect your wallet first.')
  }

  if (wallet.address !== walletAddress) {
    throw new Error('Wallet address mismatch')
  }

  try {
    const msgs = messages.map(
      (m) =>
        new MsgExecuteContract({
          sender: walletAddress,
          contract: m.contract,
          msg: m.msg,
          funds: m.coins && m.coins.length > 0 ? m.coins : [],
        })
    )

    const unsignedTx: UnsignedTx = {
      msgs,
      memo: '',
    }

    const totalGas = messages.reduce((sum, m) => sum + getGasLimitForTx(m.msg), 0)
    const fee = estimateTerraClassicFee(totalGas)

    const txHash = await wallet.broadcastTx(unsignedTx, fee)
    const { txResponse } = await wallet.pollTx(txHash)

    if (txResponse.code !== 0) {
      const errorMsg =
        txResponse.rawLog || txResponse.logs?.[0]?.log || `Transaction failed with code ${txResponse.code}`
      throw new Error(`Transaction failed: ${errorMsg}`)
    }

    return txHash
  } catch (error: unknown) {
    console.error('Terra Classic multi-message transaction error:', error)
    throw handleTransactionError(error)
  }
}

function handleTransactionError(error: unknown): Error {
  if (error instanceof Error) {
    const errorMessage = error.message

    if (
      errorMessage.includes('User rejected') ||
      errorMessage.includes('rejected') ||
      errorMessage.includes('User denied') ||
      errorMessage.includes('user rejected')
    ) {
      return new Error('Transaction rejected by user')
    }

    if (
      errorMessage.includes('Failed to fetch') ||
      errorMessage.includes('NetworkError') ||
      errorMessage.includes('network')
    ) {
      return new Error(`Network error: ${errorMessage}. Please check your internet connection and try again.`)
    }

    return new Error(`Transaction failed: ${errorMessage}`)
  }

  return new Error(`Transaction failed: ${String(error)}`)
}
