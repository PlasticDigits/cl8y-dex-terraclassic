import type { IndexerPair, IndexerTrade } from '@/types'
import { tradeToToken1PerToken0Human } from './limitOrderPriceReference'
import { invertUsd, parseFinitePositive } from './tradePairDisplayOrientation'

/** USTR/UST1 client fallback for **pre-#556** indexers only (legacy 2.5× USTC seed).
 * New indexer sends `price_usd` from DEX hub marks — prefer that field. */
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

/**
 * USD of 1 **displayed** base (GitLab #524). Factory `price_usd` is always USD of `asset_0`.
 * When inverted, prefer `price_usd / human_price`, else the quote catalog for the display leg.
 */
export function resolveDisplayTapeLastPriceUsd(
  opts: TapeUsdInput & { inverted: boolean; displayBaseSymbol?: string; displayBaseDenom?: string | null }
): string | null {
  const factory = resolveTapeLastPriceUsd(opts)
  if (!opts.inverted) return factory
  const viaRatio = invertUsd(factory, opts.price)
  if (viaRatio) return viaRatio
  const kind = classifyQuoteSymbol(opts.displayBaseSymbol ?? '', opts.displayBaseDenom)
  const catalog = quoteTokenUsd(kind, parsePositiveDecimal(opts.ustcUsd), parsePositiveDecimal(opts.luncUsd))
  return catalog == null ? null : String(catalog)
}

export type DisplayPairStatsUsdOhlc = {
  highUsd: string | null
  lowUsd: string | null
  openUsd: string | null
  closeUsd: string | null
  priceChangePct: number | null
}

/**
 * Display 24h USD OHLC + % for Charts (#680).
 * Inverted tiles use {@link invertUsd} per field (never `1/x` of factory USD) and
 * swap High/Low after invert. % is recomputed from display open→close — not
 * `-(factory price_change_pct)`.
 */
export function displayUsdPriceChangePct(
  openUsd: string | number | null | undefined,
  closeUsd: string | number | null | undefined
): number | null {
  const open = parseFinitePositive(openUsd)
  const close = parseFinitePositive(closeUsd)
  if (open == null || close == null) return null
  const pct = ((close - open) / open) * 100
  if (!Number.isFinite(pct)) return null
  return pct
}

export function resolveDisplayPairStatsUsdOhlc(args: {
  inverted: boolean
  highUsd?: string | null
  lowUsd?: string | null
  openUsd?: string | null
  closeUsd?: string | null
  highHuman?: string | null
  lowHuman?: string | null
  openHuman?: string | null
  closeHuman?: string | null
  factoryPriceChangePct?: number | null
}): DisplayPairStatsUsdOhlc {
  const factoryHigh = pairStatsUsdField(args.highUsd)
  const factoryLow = pairStatsUsdField(args.lowUsd)
  const factoryOpen = pairStatsUsdField(args.openUsd)
  const factoryClose = pairStatsUsdField(args.closeUsd)

  if (!args.inverted) {
    return {
      highUsd: factoryHigh,
      lowUsd: factoryLow,
      openUsd: factoryOpen,
      closeUsd: factoryClose,
      priceChangePct: args.factoryPriceChangePct ?? null,
    }
  }

  const invHigh = invertUsd(factoryHigh, args.highHuman)
  const invLow = invertUsd(factoryLow, args.lowHuman)
  const invOpen = invertUsd(factoryOpen, args.openHuman)
  const invClose = invertUsd(factoryClose, args.closeHuman)

  const highN = parseFinitePositive(invHigh)
  const lowN = parseFinitePositive(invLow)
  let highUsd = invHigh
  let lowUsd = invLow
  if (highN != null && lowN != null) {
    if (highN < lowN) {
      highUsd = invLow
      lowUsd = invHigh
    }
  }

  return {
    highUsd,
    lowUsd,
    openUsd: invOpen,
    closeUsd: invClose,
    priceChangePct: displayUsdPriceChangePct(invOpen, invClose),
  }
}
