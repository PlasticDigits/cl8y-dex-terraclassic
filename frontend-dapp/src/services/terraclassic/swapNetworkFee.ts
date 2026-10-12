/**
 * Swap-page network fee envelope ([GitLab #587](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/587),
 * [#679](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/679)).
 *
 * Reviewed local execution uses offlineSwapGasLimit, shared with canonical broadcast (#1480).
 * Remaining non-local hints use estimateTerraClassicFeeForEntries / getGasLimitForTx.
 * Native wrap+router uses {@link nativeSwapFeeExecuteMsgs} so Max, hint, and submit stay aligned.
 * Native path stays pool-only — do not attach hybrid here.
 * CW20 multi-hop must pass {@link SwapNetworkFeeHints.cw20RouterOperations} (indexer/submit ops)
 * so mixed hybrid hops do not fall back to pool-only `hopCount`.
 */
import type { TerraExecuteContractEntry } from '@/services/terraclassic/terraBroadcast'
import type { SwapOperation } from '@/services/terraclassic/router'
import type { AutoGasExecuteEntry } from '@/services/terraclassic/swapAutoGas'
import {
  ROUTER_CONTRACT_ADDRESS,
  TREASURY_CONTRACT_ADDRESS,
  WRAP_MAPPER_CONTRACT_ADDRESS,
  effectiveGasPriceUluna,
} from '@/utils/constants'
import { estimateFeeUlunaAmountForGasLimit, sendHookExecuteSwapOperationsMsg } from '@/services/terraclassic/terraGas'
import { offlineSwapGasLimit, OFFLINE_SWAP_BUDGET_UNAVAILABLE } from './offlineSwapBudget'
import type { HybridSwapParams } from '@/types'
import { estimateTerraClassicFeeForEntries, type TerraClassicFeeEstimate } from './terraClassicFeeEstimate'
import {
  estimateMarketPairSwapSequenceUlunaFeesTotal,
  nativeSwapFeeExecuteMsgs,
  type NativeSwapFeeHints,
} from './transactions'

export type SwapNetworkFeeHints = NativeSwapFeeHints & {
  taxFundingVersion?: 1
  /** Direct unwrap (CW20 → native via wrap-mapper, not router). */
  isDirectUnwrap?: boolean
  /** CW20↔CW20 single-hop pair `swap` (not router). */
  cw20DirectPair?: boolean
  /** Direct-pair hybrid book split is on the wire (never for wrap+multihop). */
  cw20Hybrid?: boolean
  /** Exact direct-pair book work; a boolean cannot establish its cost. */
  cw20HybridParams?: HybridSwapParams | null
  /** Reviewed routes always use the offline policy, including one legacy router hop. */
  localExecution?: boolean
  /** Exact native router operations; native composed routes must remain pool-only. */
  nativeRouterOperations?: SwapOperation[]
  /**
   * Indexer/submit `operations` for CW20 router paths (#679).
   * When `length ≥ 2`, hint uses the same `send` → `execute_swap_operations` envelope as broadcast.
   */
  cw20RouterOperations?: SwapOperation[]
}

function entriesFromMsgs(msgs: Array<{ msg: Record<string, unknown> }>): TerraExecuteContractEntry[] {
  return msgs.map((m) => ({ contract: '', msg: m.msg }))
}

/**
 * Network fee for the Swap submit path. Fee denom is always `uluna` / LUNC.
 */
export function estimateSwapNetworkFee(hints: SwapNetworkFeeHints): TerraClassicFeeEstimate {
  const entries = swapFeeBudgetEntries(hints)
  const gasLimit = hints.localExecution
    ? offlineSwapGasLimit(entries)
    : estimateTerraClassicFeeForEntries(entries).gasLimit
  return { gasLimit, feeUluna: estimateFeeUlunaAmountForGasLimit(gasLimit), gasPriceUluna: effectiveGasPriceUluna() }
}

/** Rendering must not turn an unsupported execution family into a cheap fee or crash the page. */
export function tryEstimateSwapNetworkFee(hints: SwapNetworkFeeHints): TerraClassicFeeEstimate {
  try {
    return estimateSwapNetworkFee(hints)
  } catch {
    return {
      gasLimit: 0,
      feeUluna: 0n,
      gasPriceUluna: effectiveGasPriceUluna(),
      coverageError: OFFLINE_SWAP_BUDGET_UNAVAILABLE,
    }
  }
}

/** Exact work-bearing message fields shared by the fee row and canonical broadcast classifier.
 * Amount/floor/deadline validation remains in the reviewed local submit contract. */
function swapFeeBudgetEntries(hints: SwapNetworkFeeHints): TerraExecuteContractEntry[] {
  if (hints.taxFundingVersion != null && hints.taxFundingVersion !== 1) throw new Error(OFFLINE_SWAP_BUDGET_UNAVAILABLE)
  if (hints.isDirectUnwrap)
    return entriesFromMsgs([
      {
        msg: {
          send: { msg: btoa(JSON.stringify({ unwrap_with_minimum: { recipient: null, minimum_receive: '1' } })) },
        },
      },
    ])
  if (hints.isDirectWrap) return entriesFromMsgs([{ msg: { wrap_deposit_with_minimum: { minimum_receive: '1' } } }])
  if (hints.needsWrapInput || hints.needsUnwrapOutput) {
    const operations = hints.nativeRouterOperations
    if (operations) {
      const msg = sendHookExecuteSwapOperationsMsg(
        operations.map((op) => ({ terra_swap: op.terra_swap as unknown as Record<string, unknown> })),
        { funding_version: hints.taxFundingVersion, unwrap_output: hints.needsUnwrapOutput }
      )
      return entriesFromMsgs(hints.needsWrapInput ? [{ msg: { wrap_deposit: {} } }, { msg }] : [{ msg }])
    }
    return entriesFromMsgs(nativeSwapFeeExecuteMsgs(hints))
  }
  if (hints.cw20DirectPair) {
    if (hints.cw20Hybrid && !hints.cw20HybridParams) throw new Error(OFFLINE_SWAP_BUDGET_UNAVAILABLE)
    return entriesFromMsgs([{ msg: { swap: hints.cw20HybridParams ? { hybrid: hints.cw20HybridParams } : {} } }])
  }
  const operations = hints.cw20RouterOperations
  if (operations && operations.length >= 1)
    return entriesFromMsgs([
      {
        msg: sendHookExecuteSwapOperationsMsg(
          operations.map((op) => ({ terra_swap: op.terra_swap as unknown as Record<string, unknown> })),
          { funding_version: hints.taxFundingVersion }
        ),
      },
    ])
  if (hints.localExecution) throw new Error(OFFLINE_SWAP_BUDGET_UNAVAILABLE)
  const hops = hints.hopCount ?? 1
  if (!Number.isSafeInteger(hops) || hops < 1 || hops > 4) throw new Error(OFFLINE_SWAP_BUDGET_UNAVAILABLE)
  return entriesFromMsgs(nativeSwapFeeExecuteMsgs(hints))
}

export type SwapAutoGasProbeInput = {
  hints: SwapNetworkFeeHints
  payContract?: string
  payAmount?: string
  /** Direct-pair `swap` target. CW20 router probes use the pay token as the send contract. */
  pairContract?: string
  maxSpread?: string
  minimumReceive?: string
  nativeDenom?: string
}

/**
 * Historical non-local auto-gas probe shape; production local Swap/Trade never call this helper.
 * Remaining non-local broadcast simulates the exact signed messages
 * (deadline and hop min-return enrichment can differ by a few hundred gas).
 * `max_spread` and `minimum_receive` are copied through; this helper does not drop them.
 */
export function swapAutoGasProbeEntries(input: SwapAutoGasProbeInput): AutoGasExecuteEntry[] | null {
  const spread = input.maxSpread ?? '0.05'
  const minimum = input.minimumReceive
  const amount = input.payAmount && input.payAmount !== '0' ? input.payAmount : '1'
  const hints = input.hints

  if (hints.isDirectUnwrap) {
    if (!input.payContract) return null
    return [
      {
        contract: input.payContract,
        msg: {
          send: {
            contract: WRAP_MAPPER_CONTRACT_ADDRESS,
            amount,
            msg: btoa(JSON.stringify({ unwrap_with_minimum: { recipient: null, minimum_receive: minimum ?? '1' } })),
          },
        },
      },
    ]
  }

  if (hints.isDirectWrap) {
    const denom = input.nativeDenom || 'uluna'
    return [
      {
        contract: TREASURY_CONTRACT_ADDRESS,
        msg: { wrap_deposit_with_minimum: { minimum_receive: minimum ?? '1' } },
        coins: [{ denom, amount }],
      },
    ]
  }

  if (hints.needsWrapInput || hints.needsUnwrapOutput) {
    const denom = input.nativeDenom || 'uluna'
    return nativeSwapFeeExecuteMsgs(hints).map((entry) => ({
      contract: 'wrap_deposit' in entry.msg ? TREASURY_CONTRACT_ADDRESS : (input.payContract ?? ''),
      msg: entry.msg,
      coins: 'wrap_deposit' in entry.msg ? [{ denom, amount }] : undefined,
    }))
  }

  const routerOps = hints.cw20RouterOperations
  if (
    routerOps &&
    (routerOps.length >= 2 || (hints.taxFundingVersion === 1 && routerOps.length >= 1)) &&
    input.payContract
  ) {
    const operations = routerOps.map((op) => ({
      terra_swap: op.terra_swap as unknown as Record<string, unknown>,
    }))
    const hook: Record<string, unknown> = {
      [hints.taxFundingVersion === 1 ? 'execute_route_swap_operations' : 'execute_swap_operations']: {
        ...(hints.taxFundingVersion === 1 ? { funding_version: 1 } : {}),
        operations,
        max_spread: spread,
        ...(minimum ? { minimum_receive: minimum } : {}),
      },
    }
    return [
      {
        contract: input.payContract,
        msg: {
          send: {
            contract: ROUTER_CONTRACT_ADDRESS,
            amount,
            msg: btoa(JSON.stringify(hook)),
          },
        },
      },
    ]
  }

  if (hints.cw20DirectPair && input.pairContract) {
    const swap: Record<string, unknown> = { max_spread: spread }
    if (minimum) swap.minimum_receive = minimum
    if (hints.cw20Hybrid) {
      if (!hints.cw20HybridParams) return null
      swap.hybrid = hints.cw20HybridParams
    }
    return [{ contract: input.pairContract, msg: { swap } }]
  }

  return null
}

/**
 * Trade market preflight: allowance + swap envelope.
 * Multi-hop shares {@link estimateSwapNetworkFee} (mixed-hop helper) — no second formula (#679).
 */
export function estimateTradeMarketNetworkFeeUluna(
  routerOperations: SwapOperation[] | undefined,
  hybrid?: HybridSwapParams | null,
  taxFundingVersion?: 1,
  localExecution = false
): bigint {
  if (
    routerOperations &&
    (routerOperations.length >= 2 || ((taxFundingVersion === 1 || localExecution) && routerOperations.length > 0))
  ) {
    const swap = tryEstimateSwapNetworkFee({
      isDirectWrap: false,
      needsWrapInput: false,
      hopCount: routerOperations.length,
      cw20RouterOperations: routerOperations,
      taxFundingVersion,
      localExecution,
    })
    return swap.feeUluna
  }
  return estimateMarketPairSwapSequenceUlunaFeesTotal(true, hybrid ?? undefined)
}
