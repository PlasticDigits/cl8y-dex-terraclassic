/**
 * Spread / max-spread helpers aligned with pair `assert_max_spread` / [`dex_common::max_spread`](../../smartcontracts/packages/dex-common/src/max_spread.rs) when `belief_price` is unset
 * (see `smartcontracts/contracts/pair/src/contract.rs`, invariant L9 / GitLab #197, #273, #307).
 *
 * Invariants (GitLab #134, #273, #307):
 * - `spread_cmp = min(spread_amount, pool_gross)` where `pool_gross = pool_return + commission`.
 * - `total_gross_out = pool_gross + book_return_net` (book net is 0 for pool-only hops).
 * - No-belief hybrid with both legs: `book_shortfall = max(0, pool_net * book_input / pool_input - book_net)`.
 * - No-belief hybrid with `book_input > 0` and `pool_input > 0`: `pool_input` must be ≥ 10% of offer (min 1 raw unit).
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

/** Minimum pool leg for no-belief hybrid with a book leg (GitLab #307). */
export function minPoolInputForBookHybrid(offerAmount: bigint): bigint {
  if (offerAmount <= 0n) return 0n
  const ratioMin = (offerAmount * 1n) / 10n
  return ratioMin === 0n ? 1n : ratioMin
}

export type HybridNoBeliefMaterialPoolReject =
  | { kind: 'insufficient_pool_leg'; poolInput: bigint; minPoolInput: bigint; bookInput: bigint }
  | { kind: 'zero_pool_net'; poolInput: bigint; bookInput: bigint }

/** Returns a reject reason when declared/realized hybrid legs fail the #307 floor (no belief). */
export function hybridNoBeliefMaterialPoolReject(
  offerAmount: bigint,
  poolInput: bigint,
  bookInput: bigint,
  poolNetReturn: bigint
): HybridNoBeliefMaterialPoolReject | null {
  if (bookInput === 0n || poolInput === 0n) return null
  const minPool = minPoolInputForBookHybrid(offerAmount)
  if (poolInput < minPool) {
    return { kind: 'insufficient_pool_leg', poolInput, minPoolInput: minPool, bookInput }
  }
  if (poolNetReturn === 0n) {
    return { kind: 'zero_pool_net', poolInput, bookInput }
  }
  return null
}

/** Realized pool/book legs for #273 shortfall (mirrors `execute_swap` → `assert_max_spread`). */
export function hybridMaxSpreadRealizedLegs(
  poolLeg: bigint,
  bookLeg: bigint,
  offerConsumedByBook: bigint
): { poolInput: bigint; bookInput: bigint } {
  if (bookLeg === 0n) {
    return { poolInput: poolLeg, bookInput: 0n }
  }
  return {
    poolInput: poolLeg + bookLeg - offerConsumedByBook,
    bookInput: offerConsumedByBook,
  }
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

export function hybridSpreadCmpAndTotal(
  sim: {
    spread_amount: string
    commission_amount: string
    pool_return_amount: string
    book_return_amount: string
  },
  legs?: { poolInput: bigint; bookInput: bigint }
): { spreadCmp: bigint; totalGrossOut: bigint } {
  const poolRet = BigInt(sim.pool_return_amount)
  const comm = BigInt(sim.commission_amount)
  const spr = BigInt(sim.spread_amount)
  const bookNet = BigInt(sim.book_return_amount)
  const poolGross = poolRet + comm
  let spreadCmp = spr < poolGross ? spr : poolGross
  if (legs && legs.poolInput > 0n && legs.bookInput > 0n) {
    const fairNetBook = (poolRet * legs.bookInput) / legs.poolInput
    const bookShortfall = fairNetBook > bookNet ? fairNetBook - bookNet : 0n
    spreadCmp += bookShortfall
  }
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
