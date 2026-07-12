import type { PoolResponse } from '@/types'

/**
 * First LP mint on an empty pool permanently locks this many LP **smallest units** (`MINIMUM_LIQUIDITY`
 * on the pair). The LP CW20 exposes **18** `decimals`; this constant is unchanged in raw units.
 * Mirrors `smartcontracts/contracts/pair/src/contract.rs`; see [**#124**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/124).
 */
export const PAIR_MINIMUM_LIQUIDITY = 1000n

/** Integer square root (floor). Used to mirror the pair contract’s `isqrt` for first deposit. */
export function isqrt(n: bigint): bigint {
  if (n < 0n) throw new Error('isqrt: negative')
  if (n < 2n) return n
  let x = n
  let y = (x + 1n) / 2n
  while (y < x) {
    x = y
    y = (n / x + x) / 2n
  }
  return x
}

/**
 * LP tokens the user would receive, mirroring on-chain `provide_liquidity`:
 * - **First deposit (both reserves zero):** `isqrt(amount_a × amount_b) − PAIR_MINIMUM_LIQUIDITY`
 * - **Later:** `min(amount_a × total_share / reserve_a, amount_b × total_share / reserve_b)` (floor each term)
 *
 * @returns `null` if amounts are empty/zero or the deposit would mint no user-facing LP
 */
export function estimateProvideLiquidityUserLp(
  rawAmountA: string,
  rawAmountB: string,
  pool: Pick<PoolResponse, 'assets' | 'total_share'>
): bigint | null {
  if (!rawAmountA?.trim() || !rawAmountB?.trim()) return null
  let a: bigint
  let b: bigint
  try {
    a = BigInt(rawAmountA)
    b = BigInt(rawAmountB)
  } catch {
    return null
  }
  if (a <= 0n || b <= 0n) return null

  const resA = BigInt(pool.assets[0].amount)
  const resB = BigInt(pool.assets[1].amount)
  const totalShare = BigInt(pool.total_share)

  const isFirst = resA === 0n && resB === 0n

  if (isFirst) {
    const product = a * b
    const lpTotal = isqrt(product)
    if (lpTotal <= PAIR_MINIMUM_LIQUIDITY) return null
    return lpTotal - PAIR_MINIMUM_LIQUIDITY
  }

  if (resA === 0n || resB === 0n) return null

  const lpA = (a * totalShare) / resA
  const lpB = (b * totalShare) / resB
  return lpA < lpB ? lpA : lpB
}

/**
 * Pro-rata counterpart raw amount for the other pool asset:
 * `floor(edited × reserve_other / reserve_edited)`.
 *
 * @returns `null` when pool reserves are empty, one-sided, or `editedRaw` is invalid/zero
 */
export function computeProportionalCounterpartRaw(
  editedSide: 'a' | 'b',
  editedRaw: string,
  pool: Pick<PoolResponse, 'assets'>
): string | null {
  if (!editedRaw?.trim()) return null
  let edited: bigint
  try {
    edited = BigInt(editedRaw)
  } catch {
    return null
  }
  if (edited <= 0n) return null

  const resA = BigInt(pool.assets[0].amount)
  const resB = BigInt(pool.assets[1].amount)

  if (resA === 0n && resB === 0n) return null
  if (resA === 0n || resB === 0n) return null

  if (editedSide === 'a') {
    return ((edited * resB) / resA).toString()
  }
  return ((edited * resA) / resB).toString()
}

/**
 * `true` when the two user amounts match the current pool price (so neither side is
 * “donated” to existing LPs). Uses floor-ratio match via {@link computeProportionalCounterpartRaw}.
 */
export function isProportionalAddAmounts(
  rawAmountA: string,
  rawAmountB: string,
  pool: Pick<PoolResponse, 'assets' | 'total_share'>
): boolean | null {
  if (!rawAmountA?.trim() || !rawAmountB?.trim()) return null
  let a: bigint
  let b: bigint
  try {
    a = BigInt(rawAmountA)
    b = BigInt(rawAmountB)
  } catch {
    return null
  }
  if (a <= 0n || b <= 0n) return null

  const resA = BigInt(pool.assets[0].amount)
  const resB = BigInt(pool.assets[1].amount)

  if (resA === 0n && resB === 0n) return true
  if (resA === 0n || resB === 0n) return null

  const expectedB = computeProportionalCounterpartRaw('a', rawAmountA, pool)
  if (expectedB === null) return null
  return expectedB === rawAmountB
}
