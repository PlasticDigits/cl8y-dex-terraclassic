import type { IndexerPair, IndexerTrade } from '@/types'
import { tradeToToken1PerToken0Human } from './limitOrderPriceReference'

/** USTR is 2.5× USTC on the #508 secondary AMM (matches indexer `USTR_PER_USTC`). */
export const USTR_PER_USTC = 2.5

export type QuoteUsdKind = 'ustc' | 'lunc' | 'peg1' | 'ustr' | 'unknown'

export function classifyQuoteSymbol(symbol: string, denom?: string | null): QuoteUsdKind {
  if (denom === 'uusd') return 'ustc'
  if (denom === 'uluna') return 'lunc'
  switch (symbol.trim().toUpperCase()) {
    case 'UST1':
      return 'peg1'
    case 'USTC':
    case 'CUSTC':
      return 'ustc'
    case 'LUNC':
    case 'CLUNC':
      return 'lunc'
    case 'USTR':
      return 'ustr'
    default:
      return 'unknown'
  }
}

export function quoteTokenUsd(
  kind: QuoteUsdKind,
  ustcUsd: number | null | undefined,
  luncUsd?: number | null
): number | null {
  if (kind === 'peg1') return 1
  if (kind === 'ustc') return finitePositive(ustcUsd)
  if (kind === 'lunc') return finitePositive(luncUsd)
  if (kind === 'ustr') {
    const u = finitePositive(ustcUsd)
    return u == null ? null : u * USTR_PER_USTC
  }
  return null
}

function finitePositive(n: number | null | undefined): number | null {
  if (n == null || !Number.isFinite(n) || n <= 0) return null
  return n
}

function parsePositiveDecimal(raw: string | number | null | undefined): number | null {
  if (raw == null) return null
  const n = typeof raw === 'number' ? raw : parseFloat(String(raw).trim())
  return finitePositive(n)
}

/** Pair-stats USD field: never fall back to human quote-per-base (GitLab #522). */
export function pairStatsUsdField(raw: string | null | undefined): string | null {
  const n = parsePositiveDecimal(raw)
  return n == null ? null : String(n)
}

export type TapeUsdInput = {
  priceUsd?: string | null
  /** Indexer `price` — treated as **raw** quote-per-base only when `priceUsd` is missing (pre-#522). */
  price?: string | null
  decimalsBase?: number
  decimalsQuote?: number
  quoteSymbol?: string
  quoteDenom?: string | null
  ustcUsd?: string | number | null
  luncUsd?: string | number | null
}

/**
 * USD of 1 human unit of pair base (`asset_0`).
 *
 * Prefers indexer `price_usd`. Never returns unscaled `price` as dollars.
 * Fallback (old indexer): `price × 10^(d0 − d1) × quote_usd`.
 */
export function resolveTapeLastPriceUsd(opts: TapeUsdInput): string | null {
  const indexed = parsePositiveDecimal(opts.priceUsd)
  if (indexed != null) return String(indexed)

  const raw = parsePositiveDecimal(opts.price)
  if (raw == null) return null
  const d0 = opts.decimalsBase ?? 6
  const d1 = opts.decimalsQuote ?? 6
  const human = raw * 10 ** (d0 - d1)
  if (!Number.isFinite(human) || human <= 0) return null

  const kind = classifyQuoteSymbol(opts.quoteSymbol ?? '', opts.quoteDenom)
  const ustc = parsePositiveDecimal(opts.ustcUsd)
  const lunc = parsePositiveDecimal(opts.luncUsd)
  const quoteUsd = quoteTokenUsd(kind, ustc, lunc)
  if (quoteUsd == null) return null
  return String(human * quoteUsd)
}

/**
 * Same USD as {@link resolveTapeLastPriceUsd}, preferring `trade.price_usd`,
 * else human ratio from raw amounts × quote USD (never raw `trade.price`).
 */
export function resolveTapePriceUsd(opts: {
  trade?: IndexerTrade | null
  pair?: Pick<IndexerPair, 'asset_0' | 'asset_1'> | null
  ustcUsd?: string | number | null
  luncUsd?: string | number | null
}): string | null {
  const fromField = resolveTapeLastPriceUsd({
    priceUsd: opts.trade?.price_usd,
    price: undefined,
    quoteSymbol: opts.pair?.asset_1.symbol,
    quoteDenom: opts.pair?.asset_1.denom,
    ustcUsd: opts.ustcUsd,
    luncUsd: opts.luncUsd,
  })
  if (fromField) return fromField

  const trade = opts.trade
  const pair = opts.pair
  if (!trade || !pair) return null
  const human = tradeToToken1PerToken0Human(trade, pair)
  if (human == null || !Number.isFinite(human) || human <= 0) return null
  const kind = classifyQuoteSymbol(pair.asset_1.symbol, pair.asset_1.denom)
  const quoteUsd = quoteTokenUsd(kind, parsePositiveDecimal(opts.ustcUsd), parsePositiveDecimal(opts.luncUsd))
  if (quoteUsd == null) return null
  return String(human * quoteUsd)
}
