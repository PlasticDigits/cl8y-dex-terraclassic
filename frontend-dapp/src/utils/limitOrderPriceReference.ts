import type { IndexerPair, IndexerTrade } from '@/types'

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
