/**
 * Display-only humanization for the public tape and wallet pair history (GitLab #557).
 *
 * JSON/CSV keep raw integers. Do not use these helpers for settlement or submit.
 * Invariants **T557-1–T557-11** — `docs/frontend.md` § Tape amounts and
 * `skills/AGENTS_FRONTEND_TAPE_AMOUNTS.md`.
 */

import type { IndexerLimitFill, IndexerPair, IndexerTrade } from '@/types'
import { formatPairPrice, formatTokenAmount } from '@/utils/formatAmount'
import { displayPairAssets, invertFinitePositive, parseFinitePositive } from '@/utils/tradePairDisplayOrientation'

export const TAPE_MISSING = '—'
export const TRADE_DECIMALS_MIN = 0
export const TRADE_DECIMALS_MAX = 38

export function clampTradeDecimals(raw: unknown): number | null {
  if (raw == null || raw === '') return null
  const n = typeof raw === 'number' ? raw : Number(String(raw).trim())
  if (!Number.isInteger(n) || n < TRADE_DECIMALS_MIN || n > TRADE_DECIMALS_MAX) return null
  return n
}

export function pairMatchesTrade(
  pair: IndexerPair | undefined | null,
  pairAddress: string | undefined | null
): pair is IndexerPair {
  if (!pair || !pairAddress) return false
  return pair.pair_address === pairAddress
}

function uniqueLegDecimals(pair: IndexerPair, symbol: string): number | null {
  const s0 = pair.asset_0.symbol
  const s1 = pair.asset_1.symbol
  if (!symbol || s0 === s1) return null
  if (symbol === s0) return clampTradeDecimals(pair.asset_0.decimals)
  if (symbol === s1) return clampTradeDecimals(pair.asset_1.decimals)
  return null
}

export function resolveOfferDecimals(trade: IndexerTrade, activePair?: IndexerPair | null): number | null {
  const fromApi = clampTradeDecimals(trade.offer_decimals)
  if (fromApi != null) return fromApi
  if (!pairMatchesTrade(activePair, trade.pair_address)) return null
  return uniqueLegDecimals(activePair, trade.offer_asset)
}

export function resolveAskDecimals(trade: IndexerTrade, activePair?: IndexerPair | null): number | null {
  const fromApi = clampTradeDecimals(trade.ask_decimals)
  if (fromApi != null) return fromApi
  if (!pairMatchesTrade(activePair, trade.pair_address)) return null
  return uniqueLegDecimals(activePair, trade.ask_asset)
}

export function resolveToken0Decimals(fill: IndexerLimitFill, activePair?: IndexerPair | null): number | null {
  const fromApi = clampTradeDecimals(fill.token0_decimals)
  if (fromApi != null) return fromApi
  if (!pairMatchesTrade(activePair, fill.pair_address)) return null
  return clampTradeDecimals(activePair.asset_0.decimals)
}

export function resolveToken1Decimals(fill: IndexerLimitFill, activePair?: IndexerPair | null): number | null {
  const fromApi = clampTradeDecimals(fill.token1_decimals)
  if (fromApi != null) return fromApi
  if (!pairMatchesTrade(activePair, fill.pair_address)) return null
  return clampTradeDecimals(activePair.asset_1.decimals)
}

/**
 * Human compact token amount. Missing / out-of-range decimals, non-numeric, or negative → `—`.
 * Zero with known decimals → `0` (optionally with symbol).
 */
export function formatTapeAmount(
  raw: string | undefined | null,
  decimals: number | null,
  symbol?: string | null
): string {
  if (decimals == null) return TAPE_MISSING
  if (raw == null || raw === '') return TAPE_MISSING
  // Indexer must emit plain integer digits (`bd_plain_string`). `1e+19` is not a BigInt.
  let n: bigint
  try {
    n = BigInt(raw)
  } catch {
    return TAPE_MISSING
  }
  if (n < 0n) return TAPE_MISSING
  const human = formatTokenAmount(raw, decimals)
  const trimmed = (symbol ?? '').trim()
  return trimmed ? `${human} ${trimmed}` : human
}

/** Human quote-per-base (or display reciprocal). Never USD. Non-finite / ≤0 → `—`. */
export function formatTapePrice(humanPrice: string | number | null | undefined, inverted: boolean): string {
  if (inverted) {
    const inv = invertFinitePositive(humanPrice)
    if (inv == null) return TAPE_MISSING
    return formatPairPrice(inv, 6)
  }
  const n = parseFinitePositive(humanPrice)
  if (n == null) return TAPE_MISSING
  return formatPairPrice(n, 6)
}

export function tapePriceTooltip(activePair?: IndexerPair | null, inverted = false): string {
  if (activePair) {
    const { displayBase, displayQuote } = displayPairAssets(
      activePair.asset_0.symbol,
      activePair.asset_1.symbol,
      inverted
    )
    return `Human ${displayQuote} per ${displayBase}. Not USD and not a trade instruction.`
  }
  return 'Human quote per base (asset_1 per asset_0). Not USD and not a trade instruction.'
}

/**
 * Paying display-quote (buying display-base) → true (green).
 * Paying display-base → false (red). Unknown / mixed-pair → null (no color).
 */
export function tapeRowIsBuy(trade: IndexerTrade, activePair?: IndexerPair | null, inverted = false): boolean | null {
  if (!pairMatchesTrade(activePair, trade.pair_address)) return null
  const s0 = activePair.asset_0.symbol
  const s1 = activePair.asset_1.symbol
  if (!s0 || !s1 || s0 === s1) return null
  const paying0 = trade.offer_asset === s0
  const paying1 = trade.offer_asset === s1
  if (paying0 === paying1) return null
  return inverted ? paying0 : paying1
}
