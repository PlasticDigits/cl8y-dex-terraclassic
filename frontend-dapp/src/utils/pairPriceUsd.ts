import type { IndexerPair, IndexerTrade } from '@/types'
import { tradeToToken1PerToken0Human } from './limitOrderPriceReference'
import { invertUsd } from './tradePairDisplayOrientation'
import { REGISTRY_USDT_CW20_ADDRESS } from './tokenRegistry'

export type QuoteUsdKind = 'ustc' | 'lunc' | 'peg1' | 'ustr' | 'usdt' | 'unknown'

/** Registry USDT CW20 pin (columbus-5). Symbol `USDT` alone is not a USD handle (#1258). */
export function isPinnedUsdtCw20(contract?: string | null): boolean {
  const addr = (contract ?? '').trim()
  return addr.length > 0 && addr.toLowerCase() === REGISTRY_USDT_CW20_ADDRESS.toLowerCase()
}

/**
 * Catalog leg for the price pane (#1315). Matches indexer `quote_usd_kind_for_identity`:
 * native `uusd` / `uluna`, pinned USDT, or a CW20 (contract present) whose symbol is
 * UST1 / USTC / cUSTC / LUNC / cLUNC / USTR. A native that only spoofs `USTR` is not catalog.
 */
export function isIndexerCatalogLeg(
  leg:
    | {
        symbol?: string | null
        denom?: string | null
        contract_addr?: string | null
        contractAddr?: string | null
      }
    | null
    | undefined
): boolean {
  if (!leg) return false
  const denom = (leg.denom ?? '').trim()
  if (denom === 'uusd' || denom === 'uluna') return true
  const contract = (leg.contract_addr ?? leg.contractAddr ?? '').trim()
  if (isPinnedUsdtCw20(contract)) return true
  if (!contract) return false
  switch ((leg.symbol ?? '').trim().toUpperCase()) {
    case 'UST1':
    case 'USTC':
    case 'CUSTC':
    case 'LUNC':
    case 'CLUNC':
    case 'USTR':
      return true
    default:
      return false
  }
}

export function classifyQuoteSymbol(symbol: string, denom?: string | null, contract?: string | null): QuoteUsdKind {
  if (isPinnedUsdtCw20(contract)) return 'usdt'
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

/** Strict bounded decimal input: never accept a numeric prefix of hostile text. */
function parsePositiveDecimal(raw: string | number | null | undefined): number | null {
  if (raw == null) return null
  if (typeof raw === 'string' && (raw.length > 128 || !/^(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(raw.trim())))
    return null
  const n = Number(raw)
  return Number.isFinite(n) && n > 0 ? n : null
}

/** Pair-stats USD of one human factory asset_0; human ratios are never dollars. */
export function pairStatsUsdField(raw: string | null | undefined): string | null {
  const n = parsePositiveDecimal(raw)
  return n == null ? null : String(n)
}

export type TapeUsdInput = {
  /** Indexed historical USD of one human factory asset_0, or unavailable. */
  priceUsd?: string | null
  /** Modern indexer human asset_1 per asset_0, independent of priceUsd presence. */
  price?: string | null
}

/** Modern USD contract (#1512): missing USD never selects legacy raw units or a symbol-derived mark. */
export function resolveTapeLastPriceUsd(opts: TapeUsdInput): string | null {
  return pairStatsUsdField(opts.priceUsd)
}

/** Display USD = factory asset_0 USD / human asset_1-per-asset_0, exactly once. */
export function resolveDisplayTapeLastPriceUsd(opts: TapeUsdInput & { inverted: boolean }): string | null {
  const factory = resolveTapeLastPriceUsd(opts)
  if (!opts.inverted) return factory
  const human = parsePositiveDecimal(opts.price)
  if (human == null) return null
  const result = invertUsd(factory, String(human))
  return pairStatsUsdField(result)
}

type TapePair = Pick<IndexerPair, 'pair_address' | 'asset_0' | 'asset_1'>

/** Bind the historical USD and matching human ratio to the selected pair. No current oracle fallback. */
export function resolveDisplayTapePriceUsd(opts: {
  trade?: IndexerTrade | null
  pair?: TapePair | null
  pairAddress: string
  inverted: boolean
}): string | null {
  const { trade, pair } = opts
  if (
    !trade ||
    !pair ||
    !pair.asset_0 ||
    !pair.asset_1 ||
    !opts.pairAddress ||
    pair.pair_address !== opts.pairAddress ||
    trade.pair_address !== opts.pairAddress
  )
    return null
  if (![pair.asset_0.decimals, pair.asset_1.decimals].every((d) => Number.isInteger(d) && d >= 0 && d <= 18))
    return null
  const observed = trade.block_timestamp
  if (
    typeof observed !== 'string' ||
    !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/.test(observed)
  )
    return null
  const day = Date.parse(observed.slice(0, 10))
  if (!Number.isFinite(day) || new Date(day).toISOString().slice(0, 10) !== observed.slice(0, 10)) return null
  const timestamp = Date.parse(observed)
  if (!Number.isFinite(timestamp) || timestamp > Date.now()) return null
  let human: string | undefined = trade.price
  // An absent modern ratio may be reconstructed from exact raw amounts. Invalid
  // present ratios stay invalid; missing USD still never authorizes a fallback.
  if (human == null || human === '') {
    if (
      pair.asset_0.symbol === pair.asset_1.symbol ||
      ![trade.offer_amount, trade.return_amount].every(
        (raw) => /^\d{1,39}$/.test(raw) && BigInt(raw) <= (1n << 128n) - 1n
      )
    )
      return null
    const reconstructed = tradeToToken1PerToken0Human(trade, pair)
    human = reconstructed == null ? undefined : String(reconstructed)
  }
  return resolveDisplayTapeLastPriceUsd({ inverted: opts.inverted, priceUsd: trade.price_usd, price: human })
}

/** Factory direction convenience wrapper; USD remains the indexed historical field. */
export function resolveTapePriceUsd(opts: {
  trade?: IndexerTrade | null
  pair?: TapePair | null
  pairAddress: string
}): string | null {
  return resolveDisplayTapePriceUsd({ ...opts, inverted: false })
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
  const open = parsePositiveDecimal(openUsd)
  const close = parsePositiveDecimal(closeUsd)
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
      priceChangePct: Number.isFinite(args.factoryPriceChangePct) ? args.factoryPriceChangePct! : null,
    }
  }

  const invHigh = resolveDisplayTapeLastPriceUsd({ inverted: true, priceUsd: factoryHigh, price: args.highHuman })
  const invLow = resolveDisplayTapeLastPriceUsd({ inverted: true, priceUsd: factoryLow, price: args.lowHuman })
  const invOpen = resolveDisplayTapeLastPriceUsd({ inverted: true, priceUsd: factoryOpen, price: args.openHuman })
  const invClose = resolveDisplayTapeLastPriceUsd({ inverted: true, priceUsd: factoryClose, price: args.closeHuman })

  const highN = parsePositiveDecimal(invHigh)
  const lowN = parsePositiveDecimal(invLow)
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
