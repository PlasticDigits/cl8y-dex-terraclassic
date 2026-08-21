/**
 * Swap-page network fee envelope ([GitLab #587](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/587)).
 *
 * Same math as broadcast (`estimateTerraClassicFeeForEntries` / `getGasLimitForTx`).
 * Native wrap+router uses {@link nativeSwapFeeExecuteMsgs} so Max, hint, and submit stay aligned.
 * Native path stays pool-only — do not attach hybrid here.
 */
import type { TerraExecuteContractEntry } from '@/services/terraclassic/terraBroadcast'
import {
  estimateTerraClassicFeeForEntries,
  estimateTerraClassicFeeForMsg,
  type TerraClassicFeeEstimate,
} from './terraClassicFeeEstimate'
import { nativeSwapFeeExecuteMsgs, type NativeSwapFeeHints } from './transactions'

export type SwapNetworkFeeHints = NativeSwapFeeHints & {
  /** Direct unwrap (CW20 → native via wrap-mapper, not router). */
  isDirectUnwrap?: boolean
  /** CW20↔CW20 single-hop pair `swap` (not router). */
  cw20DirectPair?: boolean
  /** Direct-pair hybrid book split is on the wire (never for wrap+multihop). */
  cw20Hybrid?: boolean
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
