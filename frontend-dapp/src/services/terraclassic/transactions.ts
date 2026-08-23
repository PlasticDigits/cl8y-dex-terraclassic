import { getConnectedWallet } from './wallet'
import type { HybridSwapParams } from '@/types'
import { hybridParamsWithSubmitCap } from './hybridSwapGas'
import {
  buildTerraClassicFee,
  estimateFeeUlunaAmountForGasLimit,
  gasLimitForLimitOrderBatch,
  getGasLimitForTx,
  totalGasLimitForExecuteMsgs,
} from './terraGas'
import {
  broadcastTerraExecuteContracts,
  type TerraBroadcastOptions,
  type TerraExecuteContractEntry,
} from './terraBroadcast'
import { getTerraBroadcastScopeOptions, withTerraBroadcastScope } from './terraBroadcastScope'

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

/** Single `update_limit_order_price` execute — no CW20 leg (GitLab #247). */
export function estimateUpdateLimitOrderPriceUlunaFeesTotal(): bigint {
  return estimateFeeUlunaAmountForGasLimit(getGasLimitForTx({ update_limit_order_price: {} }))
}

/**
 * CW20 `increase_allowance` then CW20 `send` → pair `swap` (GitLab #152 trade ticket market path).
 * Must stay aligned with {@link getGasLimitForTx} for `send` → `swap` with optional `hybrid` (GitLab #249).
 */
export function estimateMarketPairSwapSequenceUlunaFeesTotal(
  usesHybrid: boolean,
  hybridForGas?: HybridSwapParams | null
): bigint {
  const allowanceGas = getGasLimitForTx({ increase_allowance: { spender: '', amount: '' } })
  let swapInner: Record<string, unknown>
  if (usesHybrid && hybridForGas) {
    const wired = hybridParamsWithSubmitCap(hybridForGas)
    swapInner = {
      swap: {
        hybrid: {
          pool_input: wired.pool_input,
          book_input: wired.book_input,
          max_maker_fills: wired.max_maker_fills,
          book_start_hint: wired.book_start_hint ?? undefined,
        },
      },
    }
  } else if (usesHybrid) {
    swapInner = { swap: { hybrid: { pool_input: '0', book_input: '1', max_maker_fills: 8 } } }
  } else {
    swapInner = { swap: {} }
  }
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
  /** Router `unwrap_output` sub-message (CW20→native, GitLab #343). */
  needsUnwrapOutput?: boolean
  hopCount?: number
}

/**
 * Execute msgs matching `executeNativeSwap` so Max / Network fee / broadcast share one envelope
 * ([GitLab #213](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/213),
 * [#587](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/587),
 * [#599](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/599)).
 */
export function nativeSwapFeeExecuteMsgs(hints: NativeSwapFeeHints): Array<{ msg: Record<string, unknown> }> {
  if (hints.isDirectWrap) {
    return [{ msg: { wrap_deposit: {} } }]
  }

  const hopCount = Math.max(1, hints.hopCount ?? 1)
  const swapHookMsg = {
    execute_swap_operations: {
      operations: Array.from({ length: hopCount }, () => ({ terra_swap: {} })),
      max_spread: '0',
      ...(hints.needsUnwrapOutput ? { unwrap_output: true } : {}),
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
    return [{ msg: { wrap_deposit: {} } }, { msg: sendMsg }]
  }

  return [{ msg: sendMsg }]
}

/**
 * Minimum native fee (uluna) for native-input swap paths in `executeNativeSwap` ([GitLab #213](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/213)).
 * Single-tx wrap uses one envelope; wrap + router send uses summed gas like `executeTerraContractMulti`
 * (includes wrap+≥2hop combo overhead, [#587](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/587)).
 */
export function estimateNativeSwapUlunaFeesTotal(hints: NativeSwapFeeHints): bigint {
  const msgs = nativeSwapFeeExecuteMsgs(hints)
  return estimateFeeUlunaAmountForGasLimit(totalGasLimitForExecuteMsgs(msgs))
}

function encodedSend(inner: Record<string, unknown>): Record<string, unknown> {
  return {
    send: {
      contract: '',
      amount: '1',
      msg: btoa(JSON.stringify(inner)),
    },
  }
}

/**
 * Wrap + pool-only swap + allowances + provide (GitLab #533 / Z533-9). Combined multi-msg envelope.
 */
export function estimateZapInUlunaFeesTotal(opts: { wrapDeposits?: 0 | 1; routeHops?: number } = {}): bigint {
  const wrapDeposits = opts.wrapDeposits ?? 0
  const routeHops = Math.max(0, opts.routeHops ?? 0)
  const msgs: Array<{ msg: Record<string, unknown> }> = []
  for (let i = 0; i < wrapDeposits; i++) {
    msgs.push({ msg: { wrap_deposit: {} } })
  }
  if (routeHops > 0) {
    msgs.push({
      msg: encodedSend({
        execute_swap_operations: {
          operations: Array.from({ length: routeHops }, () => ({ terra_swap: {} })),
          max_spread: '0.05',
          minimum_receive: '1',
        },
      }),
    })
  }
  msgs.push({
    msg: encodedSend({
      swap: {
        max_spread: '0.05',
        min_return: '1',
        hybrid: { pool_input: '1', book_input: '0', max_maker_fills: 1 },
      },
    }),
  })
  msgs.push({ msg: { increase_allowance: { spender: '', amount: '' } } })
  msgs.push({ msg: { increase_allowance: { spender: '', amount: '' } } })
  msgs.push({ msg: { provide_liquidity: { slippage_tolerance: '0.05' } } })
  return estimateFeeUlunaAmountForGasLimit(totalGasLimitForExecuteMsgs(msgs))
}

/**
 * Withdraw + pool-only swap of the other side + optional unwrap (GitLab #533 / Z533-9).
 */
export function estimateZapOutUlunaFeesTotal(opts: { unwrap?: boolean } = {}): bigint {
  const msgs: Array<{ msg: Record<string, unknown> }> = [
    { msg: encodedSend({ withdraw_liquidity: { min_assets: ['1', '1'] } }) },
    {
      msg: encodedSend({
        swap: {
          max_spread: '0.05',
          min_return: '1',
          hybrid: { pool_input: '1', book_input: '0', max_maker_fills: 1 },
        },
      }),
    },
  ]
  if (opts.unwrap) {
    msgs.push({ msg: encodedSend({ unwrap: { recipient: null } }) })
  }
  return estimateFeeUlunaAmountForGasLimit(totalGasLimitForExecuteMsgs(msgs))
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
  const broadcastOptions = getTerraBroadcastScopeOptions()
  await executeTerraContract(
    walletAddress,
    tokenAddress,
    {
      increase_allowance: { spender, amount: amountRaw },
    },
    undefined,
    broadcastOptions
  )
  if (broadcastOptions) {
    return withTerraBroadcastScope(broadcastOptions, runAfterAllowance)
  }
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
  coins?: Array<{ denom: string; amount: string }>,
  broadcastOptions?: TerraBroadcastOptions
): Promise<string> {
  const wallet = requireConnectedWalletForAddress(walletAddress)
  return broadcastTerraExecuteContracts(
    wallet,
    walletAddress,
    [{ contract: contractAddress, msg: executeMsg, coins }],
    broadcastOptions ?? getTerraBroadcastScopeOptions()
  )
}

export async function executeTerraContractMulti(
  walletAddress: string,
  messages: TerraExecuteContractEntry[],
  broadcastOptions?: TerraBroadcastOptions
): Promise<string> {
  const wallet = requireConnectedWalletForAddress(walletAddress)
  return broadcastTerraExecuteContracts(
    wallet,
    walletAddress,
    messages,
    broadcastOptions ?? getTerraBroadcastScopeOptions()
  )
}

export type { TerraBroadcastOptions, TerraBroadcastPhase } from './terraBroadcast'

/** @internal Exported for tests that assert fee gas limits. */
export { buildTerraClassicFee, getGasLimitForTx }
