/**
 * Swap-page network fee envelope ([GitLab #587](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/587),
 * [#679](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/679)).
 *
 * Same math as broadcast (`estimateTerraClassicFeeForEntries` / `getGasLimitForTx`).
 * Native wrap+router uses {@link nativeSwapFeeExecuteMsgs} so Max, hint, and submit stay aligned.
 * Native path stays pool-only — do not attach hybrid here.
 * CW20 multi-hop must pass {@link SwapNetworkFeeHints.cw20RouterOperations} (indexer/submit ops)
 * so mixed hybrid hops do not fall back to pool-only `hopCount`.
 */
import type { TerraExecuteContractEntry } from '@/services/terraclassic/terraBroadcast'
import type { SwapOperation } from '@/services/terraclassic/router'
import {
  estimateFeeUlunaAmountForGasLimit,
  getGasLimitForTx,
  sendHookExecuteSwapOperationsMsg,
} from '@/services/terraclassic/terraGas'
import type { HybridSwapParams } from '@/types'
import {
  estimateTerraClassicFeeForEntries,
  estimateTerraClassicFeeForMsg,
  type TerraClassicFeeEstimate,
} from './terraClassicFeeEstimate'
import {
  estimateMarketPairSwapSequenceUlunaFeesTotal,
  nativeSwapFeeExecuteMsgs,
  type NativeSwapFeeHints,
} from './transactions'

export type SwapNetworkFeeHints = NativeSwapFeeHints & {
  /** Direct unwrap (CW20 → native via wrap-mapper, not router). */
  isDirectUnwrap?: boolean
  /** CW20↔CW20 single-hop pair `swap` (not router). */
  cw20DirectPair?: boolean
  /** Direct-pair hybrid book split is on the wire (never for wrap+multihop). */
  cw20Hybrid?: boolean
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
  if (hints.isDirectUnwrap) {
    return estimateTerraClassicFeeForMsg({
      send: { msg: btoa(JSON.stringify({ unwrap: { recipient: null } })) },
    })
  }

  if (hints.isDirectWrap || hints.needsWrapInput || hints.needsUnwrapOutput) {
    return estimateTerraClassicFeeForEntries(entriesFromMsgs(nativeSwapFeeExecuteMsgs(hints)))
  }

  if (hints.cw20DirectPair) {
    if (hints.cw20Hybrid) {
      return estimateTerraClassicFeeForMsg({
        swap: { hybrid: { pool_input: '1', book_input: '0', max_maker_fills: 1 } },
      })
    }
    return estimateTerraClassicFeeForMsg({ swap: {} })
  }

  const routerOps = hints.cw20RouterOperations
  if (routerOps && routerOps.length >= 2) {
    const operations = routerOps.map((op) => ({
      terra_swap: op.terra_swap as unknown as Record<string, unknown>,
    }))
    return estimateTerraClassicFeeForMsg(sendHookExecuteSwapOperationsMsg(operations))
  }

  const hopCount = Math.max(1, hints.hopCount ?? 1)
  return estimateTerraClassicFeeForEntries(
    entriesFromMsgs(
      nativeSwapFeeExecuteMsgs({
        isDirectWrap: false,
        needsWrapInput: false,
        needsUnwrapOutput: false,
        hopCount,
      })
    )
  )
}

/**
 * Trade market preflight: allowance + swap envelope.
 * Multi-hop shares {@link estimateSwapNetworkFee} (mixed-hop helper) — no second formula (#679).
 */
export function estimateTradeMarketNetworkFeeUluna(
  routerOperations: SwapOperation[] | undefined,
  hybrid?: HybridSwapParams | null
): bigint {
  if (routerOperations && routerOperations.length >= 2) {
    const swap = estimateSwapNetworkFee({
      isDirectWrap: false,
      needsWrapInput: false,
      hopCount: routerOperations.length,
      cw20RouterOperations: routerOperations,
    })
    const allowance = estimateFeeUlunaAmountForGasLimit(
      getGasLimitForTx({ increase_allowance: { spender: '', amount: '' } })
    )
    return allowance + swap.feeUluna
  }
  return estimateMarketPairSwapSequenceUlunaFeesTotal(true, hybrid ?? undefined)
}
