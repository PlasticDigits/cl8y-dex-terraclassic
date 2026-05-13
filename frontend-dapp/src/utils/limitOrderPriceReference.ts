import type { IndexerPair, IndexerTrade, PairInfo, PoolResponse } from '@/types'
import { lookupByAssetInfo } from '@/utils/tokenRegistry'

/**
 * Latest indexed swap ratio as **human** token1 per token0 (same convention as on-chain limits and `docs/limit-orders.md`).
 * Uses raw `offer_amount` / `return_amount` and pair asset decimals from the indexer row.
 */
export function tradeToToken1PerToken0Human(
  trade: IndexerTrade,
  pair: Pick<IndexerPair, 'asset_0' | 'asset_1'>
): number | null {
  const d0 = pair.asset_0.decimals
  const d1 = pair.asset_1.decimals
  const s0 = pair.asset_0.symbol
  const s1 = pair.asset_1.symbol

  let offerRaw: bigint
  let returnRaw: bigint
  try {
    offerRaw = BigInt(trade.offer_amount.split('.')[0] || '0')
    returnRaw = BigInt(trade.return_amount.split('.')[0] || '0')
  } catch {
    return null
  }
  if (offerRaw <= 0n || returnRaw <= 0n) return null

  if (trade.offer_asset === s0 && trade.ask_asset === s1) {
    const num = returnRaw * 10n ** BigInt(d0)
    const den = offerRaw * 10n ** BigInt(d1)
    if (den === 0n) return null
    return bigRatioToNumber(num, den)
  }
  if (trade.offer_asset === s1 && trade.ask_asset === s0) {
    const num = offerRaw * 10n ** BigInt(d0)
    const den = returnRaw * 10n ** BigInt(d1)
    if (den === 0n) return null
    return bigRatioToNumber(num, den)
  }
  return null
}

function bigRatioToNumber(num: bigint, den: bigint): number {
  const n = Number(num)
  const d = Number(den)
  if (!Number.isFinite(n) || !Number.isFinite(d)) return Number.NaN
  if (d === 0) return Number.NaN
  return n / d
}

/** Where the limit-order reference token1/token0 rate came from (GitLab #166). */
export type LimitOrderPriceRefSource = 'tape' | 'pool'

/**
 * Decimals for pair asset0 / asset1: indexer row when present, else CW20/native registry only.
 * Returns null if decimals cannot be determined without guessing (unknown CW20 not in registry).
 */
export function pairDecimalsForLimitPriceRef(
  indexerPair: Pick<IndexerPair, 'asset_0' | 'asset_1'> | null | undefined,
  pairInfo: PairInfo | null | undefined
): { d0: number; d1: number } | null {
  if (indexerPair) {
    return { d0: indexerPair.asset_0.decimals, d1: indexerPair.asset_1.decimals }
  }
  if (!pairInfo) return null
  const r0 = lookupByAssetInfo(pairInfo.asset_infos[0])
  const r1 = lookupByAssetInfo(pairInfo.asset_infos[1])
  if (r0 == null || r1 == null) return null
  return { d0: r0.decimals, d1: r1.decimals }
}

/**
 * Constant-product **spot** token1 per token0 from pair `pool` reserves (human-scale).
 * Matches pair ordering: `assets[0]` = token0, `assets[1]` = token1.
 */
export function poolReservesToToken1PerToken0Human(
  pool: Pick<PoolResponse, 'assets'>,
  decimals0: number,
  decimals1: number
): number | null {
  const [a0, a1] = pool.assets
  let r0: bigint
  let r1: bigint
  try {
    r0 = BigInt(String(a0.amount).split('.')[0] || '0')
    r1 = BigInt(String(a1.amount).split('.')[0] || '0')
  } catch {
    return null
  }
  if (r0 <= 0n || r1 <= 0n) return null
  const num = r1 * 10n ** BigInt(decimals0)
  const den = r0 * 10n ** BigInt(decimals1)
  if (den === 0n) return null
  const n = Number(num)
  const d = Number(den)
  if (!Number.isFinite(n) || !Number.isFinite(d) || d === 0) return null
  const x = n / d
  return x > 0 && Number.isFinite(x) ? x : null
}

export type ResolveLimitOrderPriceRefResult = {
  refToken1PerToken0: number | null
  refSource: LimitOrderPriceRefSource | null
}

/**
 * Prefer indexed **last trade**; when missing or unparseable, fall back to on-chain **pool** spot
 * so bid/ask direction checks still run when the indexer is down (GitLab #166).
 */
export function resolveLimitOrderPriceRef(input: {
  latestTrade: IndexerTrade | null | undefined
  indexerPair: Pick<IndexerPair, 'asset_0' | 'asset_1'> | null | undefined
  pool: Pick<PoolResponse, 'assets'> | null | undefined
  pairInfo: PairInfo | null | undefined
}): ResolveLimitOrderPriceRefResult {
  const { latestTrade, indexerPair, pool, pairInfo } = input
  if (latestTrade && indexerPair) {
    const t = tradeToToken1PerToken0Human(latestTrade, indexerPair)
    if (t != null && t > 0 && Number.isFinite(t)) {
      return { refToken1PerToken0: t, refSource: 'tape' }
    }
  }
  const dec = pairDecimalsForLimitPriceRef(indexerPair, pairInfo)
  if (!pool || !dec) {
    return { refToken1PerToken0: null, refSource: null }
  }
  const p = poolReservesToToken1PerToken0Human(pool, dec.d0, dec.d1)
  if (p == null) {
    return { refToken1PerToken0: null, refSource: null }
  }
  return { refToken1PerToken0: p, refSource: 'pool' }
}

/** True when an indexed trade row yields a finite positive reference without using the pool. */
export function hasResolvableTapeRef(
  latestTrade: IndexerTrade | null | undefined,
  indexerPair: Pick<IndexerPair, 'asset_0' | 'asset_1'> | null | undefined
): boolean {
  if (!latestTrade || !indexerPair) return false
  const t = tradeToToken1PerToken0Human(latestTrade, indexerPair)
  return t != null && t > 0 && Number.isFinite(t)
}

export function parsePositivePriceHuman(s: string): number | null {
  const t = s.trim()
  if (!t) return null
  const n = Number(t)
  if (!Number.isFinite(n) || n <= 0) return null
  return n
}

/**
 * Buy limits must be **below** the reference; sell limits **above** (GitLab #154).
 * Equality (`===`) is treated as invalid — at-the-print fills like a market order for retail safety.
 */
export function isLimitPriceDirectionInvalid(
  side: 'bid' | 'ask',
  limitHuman: number,
  refToken1PerToken0: number
): boolean {
  if (!(refToken1PerToken0 > 0) || !(limitHuman > 0)) return false
  if (side === 'bid') return limitHuman >= refToken1PerToken0
  return limitHuman <= refToken1PerToken0
}

/** Signed percent vs reference: `(limit - ref) / ref * 100`. */
export function limitPriceDeviationPercent(limitHuman: number, refToken1PerToken0: number): number | null {
  if (!(refToken1PerToken0 > 0) || !Number.isFinite(limitHuman)) return null
  return ((limitHuman - refToken1PerToken0) / refToken1PerToken0) * 100
}

/**
 * Anchor-scaled “chart headline” USD for the typed limit price: when `limit === ref`, returns `headlineUsd`.
 * Uses the same tape/candle headline number the trade page already passes into `PriceChart` (see `docs/frontend.md` § Trade page — market context).
 */
export function anchorUsdForLimitPrice(
  limitHuman: number,
  refToken1PerToken0: number,
  headlineUsd: string | null | undefined
): number | null {
  if (!(refToken1PerToken0 > 0) || !(limitHuman > 0)) return null
  const h = parseFloat(String(headlineUsd ?? '').trim())
  if (!Number.isFinite(h) || h <= 0) return null
  return (limitHuman / refToken1PerToken0) * h
}
