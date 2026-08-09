/**
 * Client-side UST1 window quote math (GitLab #506).
 *
 * Rate and fee come from on-chain `effective_swap` — never invent rates.
 * Integer math mirrors ust1-common INV-SWAP-001 / INV-SWAP-002
 * (PlasticDigits/ust1-window `ust1_common::math`).
 */

export const UST1_RATE_SCALE = 1_000_000_000_000_000_000n // 1e18
export const UST1_BPS_DENOM = 10_000n
/** Haircut on withdraw `min_vfdusd_out` vs quoted out (basis points). */
export const UST1_WITHDRAW_MIN_OUT_SLIPPAGE_BPS = 100

/** Apply fee on UST1 notional: `amount * (BPS_DENOM - fee_bps) / BPS_DENOM`. */
export function applyFeeUst1(amount: bigint, feeBps: number): bigint {
  if (feeBps < 0 || feeBps > Number(UST1_BPS_DENOM)) {
    throw new Error('Invalid fee_bps')
  }
  return (amount * (UST1_BPS_DENOM - BigInt(feeBps))) / UST1_BPS_DENOM
}

/** INV-SWAP-001: vFDUSD → UST1 after fee on UST1 output. */
export function depositVfdusdToUst1(amountVfdusd: bigint, rate: bigint, feeBps: number): bigint {
  const before = (amountVfdusd * rate) / UST1_RATE_SCALE
  return applyFeeUst1(before, feeBps)
}

/** INV-SWAP-002: gross UST1 → vFDUSD after fee then rate division. */
export function withdrawGrossUst1ToVfdusd(grossUst1: bigint, rate: bigint, feeBps: number): bigint {
  if (rate === 0n) throw new Error('Division by zero')
  const afterFee = applyFeeUst1(grossUst1, feeBps)
  return (afterFee * UST1_RATE_SCALE) / rate
}

export function minVfdusdOutAfterSlippage(
  quotedVfdusdOut: bigint,
  slippageBps: number = UST1_WITHDRAW_MIN_OUT_SLIPPAGE_BPS
): bigint {
  if (slippageBps < 0 || slippageBps > Number(UST1_BPS_DENOM)) {
    throw new Error('Invalid slippage bps')
  }
  return (quotedVfdusdOut * (UST1_BPS_DENOM - BigInt(slippageBps))) / UST1_BPS_DENOM
}

/** String wrappers for service / tests that prefer decimal-string chain amounts. */
export function quoteDepositUst1Out(amountVfdusd: string, rate: string, feeBps: number): string | null {
  try {
    return depositVfdusdToUst1(BigInt(amountVfdusd), BigInt(rate), feeBps).toString()
  } catch {
    return null
  }
}

export function quoteWithdrawVfdusdOut(grossUst1: string, rate: string, feeBps: number): string | null {
  try {
    return withdrawGrossUst1ToVfdusd(BigInt(grossUst1), BigInt(rate), feeBps).toString()
  } catch {
    return null
  }
}
