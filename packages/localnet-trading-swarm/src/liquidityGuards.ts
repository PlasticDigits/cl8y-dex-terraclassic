import type { PoolResponse } from './types.js'
import { assetInfoLabel } from './types.js'

/**
 * On-chain pair first-deposit lock (see `smartcontracts/contracts/pair` and
 * `frontend-dapp/src/utils/provideLiquidityEstimate.ts`). Used only for heuristics — the chain enforces truth.
 */
export const PAIR_MINIMUM_LIQUIDITY = 1000n

/**
 * Minimum **each** reserve (raw units) before we attempt swaps that would materially move the pool.
 * Chosen well below `deploy-dex-local.sh` seed liquidity (typically 1e9–1e11 per side) so the default
 * local graph stays tradeable; raises only on broken or drained deployments.
 */
export const MIN_RESERVE_PER_SIDE_FOR_SWAP = 10_000_000n

/**
 * Minimum raw amount for each leg of `provide_liquidity` from bots so the minted LP (after the 1000 lock on first deposit) stays meaningful.
 */
export const MIN_PROVIDE_LIQUIDITY_LEG = 5_000_000n

/**
 * Minimum CW20 `send` amount for swaps / hybrid legs so maker-fee dust does not reject limit placement.
 */
export const MIN_SWAP_OR_ESCROW_AMOUNT = 500_000n

export function poolReservesOk(pool: PoolResponse): boolean {
  const [a, b] = pool.assets
  try {
    const ra = BigInt(a.amount)
    const rb = BigInt(b.amount)
    return ra >= MIN_RESERVE_PER_SIDE_FOR_SWAP && rb >= MIN_RESERVE_PER_SIDE_FOR_SWAP
  } catch {
    return false
  }
}

/** True if proportional add amounts are plausibly above first-deposit / rounding noise. */
export function provideAmountsReasonable(amountA: bigint, amountB: bigint): boolean {
  return amountA >= MIN_PROVIDE_LIQUIDITY_LEG && amountB >= MIN_PROVIDE_LIQUIDITY_LEG
}

export function pickScaledProvideAmounts(
  pool: PoolResponse,
  maxFractionPpm: bigint
): { amountA: string; amountB: string } | null {
  const [pa, pb] = pool.assets
  const ra = BigInt(pa.amount)
  const rb = BigInt(pb.amount)
  if (!poolReservesOk(pool)) return null

  const fa = (ra * maxFractionPpm) / 1_000_000n
  const fb = (rb * maxFractionPpm) / 1_000_000n
  if (fa < MIN_PROVIDE_LIQUIDITY_LEG || fb < MIN_PROVIDE_LIQUIDITY_LEG) return null

  return { amountA: fa.toString(), amountB: fb.toString() }
}

export function tokenAddrsForPair(pool: PoolResponse): [string, string] {
  const t0 = assetInfoLabel(pool.assets[0].info)
  const t1 = assetInfoLabel(pool.assets[1].info)
  if (!t0.startsWith('terra1') || !t1.startsWith('terra1')) {
    throw new Error('liquidityGuards: expected CW20 pair reserves for bot provide/withdraw')
  }
  return [t0, t1]
}
