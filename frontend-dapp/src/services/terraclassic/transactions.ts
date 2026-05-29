import { getConnectedWallet } from './wallet'
import {
  buildTerraClassicFee,
  estimateFeeUlunaAmountForGasLimit,
  gasLimitForLimitOrderBatch,
  getGasLimitForTx,
  totalGasLimitForExecuteMsgs,
} from './terraGas'
import { broadcastTerraExecuteContracts, type TerraExecuteContractEntry } from './terraBroadcast'

function requireConnectedWalletForAddress(walletAddress: string) {
  const wallet = getConnectedWallet()
  if (!wallet) {
    throw new Error('Wallet not connected. Please connect your wallet first.')
  }
  if (wallet.address !== walletAddress) {
    throw new Error('Wallet address mismatch')
  }
  return wallet
}

/**
 * Minimum native fee (uluna) for the two-step CW20 limit place path: `increase_allowance` then
 * `send` → `place_limit_order`. Used for UI preflight so tx1 is not broadcast if the wallet cannot
 * pay tx2 ([GitLab #132](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/132)).
 * Must stay aligned with {@link getGasLimitForTx} for those message shapes.
 */
export function estimateLimitOrderPlaceSequenceUlunaFeesTotal(rungCount = 1): bigint {
  const allowanceGas = getGasLimitForTx({ increase_allowance: { spender: '', amount: '' } })
  const placeGas = gasLimitForLimitOrderBatch(rungCount)
  return estimateFeeUlunaAmountForGasLimit(allowanceGas) + estimateFeeUlunaAmountForGasLimit(placeGas)
}

/** Batch/ladder place: one allowance + one CW20 send (GitLab #206). */
export function estimateLimitOrderBatchPlaceSequenceUlunaFeesTotal(rungCount: number): bigint {
  return estimateLimitOrderPlaceSequenceUlunaFeesTotal(rungCount)
}

/**
 * CW20 `increase_allowance` then CW20 `send` → pair `swap` (GitLab #152 trade ticket market path).
 * Must stay aligned with {@link getGasLimitForTx} for `send` → `swap` with optional `hybrid`.
 */
export function estimateMarketPairSwapSequenceUlunaFeesTotal(usesHybrid: boolean): bigint {
  const allowanceGas = getGasLimitForTx({ increase_allowance: { spender: '', amount: '' } })
  const swapInner = usesHybrid
    ? { swap: { hybrid: { pool_input: '0', book_input: '0', max_maker_fills: 1 } } }
    : { swap: {} }
  const swapGas = getGasLimitForTx({
    send: { contract: '', amount: '', msg: btoa(JSON.stringify(swapInner)) },
  })
  return estimateFeeUlunaAmountForGasLimit(allowanceGas) + estimateFeeUlunaAmountForGasLimit(swapGas)
}

/**
 * Minimum native fee (uluna) for the three-step CW20/CW20 provide path in `provideLiquidity` (`pair.ts`):
 * two `increase_allowance` txs then `provide_liquidity`. Used so the first allowance is not broadcast if the
 * wallet cannot pay the remaining fees ([GitLab #147](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/147)).
 * Must stay aligned with {@link getGasLimitForTx} for those message shapes.
 */
export function estimateProvideLiquidityCw20SequenceUlunaFeesTotal(): bigint {
  const allowanceGas = getGasLimitForTx({ increase_allowance: { spender: '', amount: '' } })
  const provideGas = getGasLimitForTx({ provide_liquidity: {} })
  return estimateFeeUlunaAmountForGasLimit(allowanceGas) * 2n + estimateFeeUlunaAmountForGasLimit(provideGas)
}

export type NativeSwapFeeHints = {
  isDirectWrap: boolean
  needsWrapInput: boolean
  hopCount?: number
}

/**
 * Minimum native fee (uluna) for native-input swap paths in `executeNativeSwap` ([GitLab #213](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/213)).
 * Single-tx wrap uses one envelope; wrap + router send uses summed gas like `executeTerraContractMulti`.
 */
export function estimateNativeSwapUlunaFeesTotal(hints: NativeSwapFeeHints): bigint {
  if (hints.isDirectWrap) {
    return estimateFeeUlunaAmountForGasLimit(getGasLimitForTx({ wrap_deposit: {} }))
  }

  const hopCount = Math.max(1, hints.hopCount ?? 1)
  const swapHookMsg = {
    execute_swap_operations: {
      operations: Array.from({ length: hopCount }, () => ({ terra_swap: {} })),
      max_spread: '0',
    },
  }
  const sendMsg = {
    send: {
      contract: '',
      amount: '',
      msg: btoa(JSON.stringify(swapHookMsg)),
    },
  }

  if (hints.needsWrapInput) {
    const msgs = [{ msg: { wrap_deposit: {} } }, { msg: sendMsg }]
    return estimateFeeUlunaAmountForGasLimit(totalGasLimitForExecuteMsgs(msgs))
  }

  return estimateFeeUlunaAmountForGasLimit(getGasLimitForTx(sendMsg))
}

/**
 * Native wrap + provide liquidity combined tx (`PoolPage` multi-msg path, [GitLab #213](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/213)).
 */
export function estimateProvideLiquidityNativeWrapUlunaFeesTotal(wrapDepositCount: 1 | 2 = 1): bigint {
  const msgs: Array<{ msg: Record<string, unknown> }> = []
  for (let i = 0; i < wrapDepositCount; i++) {
    msgs.push({ msg: { wrap_deposit: {} } })
  }
  msgs.push({ msg: { increase_allowance: { spender: '', amount: '' } } })
  msgs.push({ msg: { increase_allowance: { spender: '', amount: '' } } })
  msgs.push({ msg: { provide_liquidity: {} } })
  return estimateFeeUlunaAmountForGasLimit(totalGasLimitForExecuteMsgs(msgs))
}

/**
 * Two-step CW20 path used by limit place and market swap tickets: `increase_allowance` then caller action.
 * Both txs use {@link broadcastTerraExecuteContracts} (GitLab #127).
 */
export async function executeCw20AllowanceThen(
  walletAddress: string,
  tokenAddress: string,
  spender: string,
  amountRaw: string,
  runAfterAllowance: () => Promise<string>
): Promise<string> {
  await executeTerraContract(walletAddress, tokenAddress, {
    increase_allowance: { spender, amount: amountRaw },
  })
  return runAfterAllowance()
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
  const wallet = requireConnectedWalletForAddress(walletAddress)
  return broadcastTerraExecuteContracts(wallet, walletAddress, [{ contract: contractAddress, msg: executeMsg, coins }])
}

export async function executeTerraContractMulti(
  walletAddress: string,
  messages: TerraExecuteContractEntry[]
): Promise<string> {
  const wallet = requireConnectedWalletForAddress(walletAddress)
  return broadcastTerraExecuteContracts(wallet, walletAddress, messages)
}

/** @internal Exported for tests that assert fee gas limits. */
export { buildTerraClassicFee, getGasLimitForTx }
