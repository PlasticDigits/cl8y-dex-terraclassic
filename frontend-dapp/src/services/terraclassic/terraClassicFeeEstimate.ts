import { effectiveGasPriceUluna } from '@/utils/constants'
import { formatTokenAmount } from '@/utils/formatAmount'
import type { TerraExecuteContractEntry } from '@/services/terraclassic/terraBroadcast'
import { estimateFeeUlunaAmountForGasLimit, getGasLimitForTx, totalGasLimitForExecuteMsgs } from './terraGas'

/**
 * Single source for Terra Classic tx fee envelopes (gas limit × {@link effectiveGasPriceUluna}).
 * Static per-message envelope (broadcast fallback and the fee row before a simulate resolves).
 * Non-local swap simulation may replace this gas with `ceil(gas_used × 1.2)` (#1360).
 * Reviewed local entries use the stricter shared offlineSwapGasLimit policy (#1480), without I/O.
 */
export type TerraClassicFeeEstimate = {
  gasLimit: number
  feeUluna: bigint
  gasPriceUluna: number
  /** Unknown/over-limit local execution families retain the quote but cannot sign. */
  coverageError?: string
}

export function estimateTerraClassicFeeForMsg(msg: Record<string, unknown>): TerraClassicFeeEstimate {
  const gasLimit = getGasLimitForTx(msg)
  return {
    gasLimit,
    feeUluna: estimateFeeUlunaAmountForGasLimit(gasLimit),
    gasPriceUluna: effectiveGasPriceUluna(),
  }
}

export function estimateTerraClassicFeeForEntries(entries: TerraExecuteContractEntry[]): TerraClassicFeeEstimate {
  const gasLimit = entries.length === 1 ? getGasLimitForTx(entries[0].msg) : totalGasLimitForExecuteMsgs(entries)
  return {
    gasLimit,
    feeUluna: estimateFeeUlunaAmountForGasLimit(gasLimit),
    gasPriceUluna: effectiveGasPriceUluna(),
  }
}

/** Human LUNC for UI copy (6 decimals). */
export function formatTerraClassicFeeLunc(feeUluna: bigint, maxFractionDigits = 4): string {
  return formatTokenAmount(feeUluna.toString(), 6, maxFractionDigits)
}
