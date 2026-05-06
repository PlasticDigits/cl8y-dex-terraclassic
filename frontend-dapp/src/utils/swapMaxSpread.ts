/**
 * Spread / max-spread helpers aligned with pair `assert_max_spread` when `belief_price` is unset
 * (see `smartcontracts/contracts/pair/src/contract.rs`).
 *
 * Invariants (GitLab #134):
 * - `spread_cmp = min(spread_amount, pool_gross)` where `pool_gross = pool_return + commission`.
 * - `total_gross_out = pool_gross + book_return_net` (book net is 0 for pool-only hops).
 * - A hop **fails** on-chain if `spread_cmp / total_gross_out > max_spread` (strict `>`), same as the contract.
 */

/** Parse a non-negative decimal string (e.g. `"0.01"`) into `mantissa / 10^scale`. */
export function parseDecimalStringToScaled(s: string): { mantissa: bigint; scalePow: bigint } {
  const trimmed = s.trim()
  const m = /^(\d+)(?:\.(\d+))?$/.exec(trimmed)
  if (!m) throw new Error(`Invalid decimal string: ${s}`)
  const intPart = m[1] || '0'
  let frac = m[2] ?? ''
  frac = frac.replace(/0+$/, '')
  const scalePow = 10n ** BigInt(frac.length)
  const mantissa = BigInt(intPart) * scalePow + (frac ? BigInt(frac) : 0n)
  return { mantissa, scalePow }
}

/** `spread_cmp / total_gross_out > maxSpreadStr` using exact bigint cross-multiply. */
export function spreadRatioStrictlyExceedsMax(spreadCmp: bigint, totalGrossOut: bigint, maxSpreadStr: string): boolean {
  if (totalGrossOut <= 0n) return false
  const { mantissa: maxNum, scalePow: maxDen } = parseDecimalStringToScaled(maxSpreadStr)
  if (maxNum <= 0n) return false
  return spreadCmp * maxDen > totalGrossOut * maxNum
}

export function poolOnlySpreadCmpAndTotal(sim: {
  return_amount: string
  spread_amount: string
  commission_amount: string
}): { spreadCmp: bigint; totalGrossOut: bigint } {
  const ret = BigInt(sim.return_amount)
  const comm = BigInt(sim.commission_amount)
  const spr = BigInt(sim.spread_amount)
  const poolGross = ret + comm
  const spreadCmp = spr < poolGross ? spr : poolGross
  return { spreadCmp, totalGrossOut: poolGross }
}

export function hybridSpreadCmpAndTotal(sim: {
  spread_amount: string
  commission_amount: string
  pool_return_amount: string
  book_return_amount: string
}): { spreadCmp: bigint; totalGrossOut: bigint } {
  const poolRet = BigInt(sim.pool_return_amount)
  const comm = BigInt(sim.commission_amount)
  const spr = BigInt(sim.spread_amount)
  const bookNet = BigInt(sim.book_return_amount)
  const poolGross = poolRet + comm
  const spreadCmp = spr < poolGross ? spr : poolGross
  const totalGrossOut = poolGross + bookNet
  return { spreadCmp, totalGrossOut }
}

/** Percent string fixed to 2 decimals: `(spreadCmp / totalGrossOut) * 100`. */
export function spreadPercentOfGross(spreadCmp: bigint, totalGrossOut: bigint): string {
  if (totalGrossOut <= 0n) return '0.00'
  const basisPoints = (spreadCmp * 10000n) / totalGrossOut
  const whole = basisPoints / 100n
  const frac = basisPoints % 100n
  return `${whole}.${frac.toString().padStart(2, '0')}`
}
