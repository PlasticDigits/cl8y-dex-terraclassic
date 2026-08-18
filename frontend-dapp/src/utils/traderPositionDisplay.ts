import type { IndexerPosition } from '@/types'
import { formatNum, formatPairPrice } from '@/utils/formatAmount'
import { classifyQuoteSymbol, quoteTokenUsd } from '@/utils/pairPriceUsd'

/** Missing / unscalable display (GitLab #551). Never show raw mixed units as a total. */
export const TRADER_PNL_EM_DASH = '—'

export type TraderOracleUsd = {
  ustcUsd?: number | null
  luncUsd?: number | null
}

/**
 * Multiply a NUMERIC string by `10^exp` (`exp` may be negative).
 * Used to humanize indexer raw token amounts and avg-entry ratios (GitLab #551).
 */
export function multiplyNumericByTenPow(raw: string, exp: number): string | null {
  const parsed = parseNumericParts(raw)
  if (parsed == null) return null
  if (!Number.isInteger(exp) || exp < -38 || exp > 38) return null
  const { neg, combined, fracLen } = parsed
  const newFrac = fracLen - exp
  let digits = combined
  let frac = newFrac
  if (frac < 0) {
    digits = digits + '0'.repeat(-frac)
    frac = 0
  } else if (digits.length <= frac) {
    digits = digits.padStart(frac + 1, '0')
  }
  const split = digits.length - frac
  const whole = digits.slice(0, split).replace(/^0+/, '') || '0'
  const fracPart = digits.slice(split).replace(/0+$/, '')
  const out = fracPart ? `${whole}.${fracPart}` : whole
  if (out === '0') return '0'
  return neg ? `-${out}` : out
}

/** `raw / 10^decimals` for signed indexer NUMERIC amounts. */
export function scaleNumericByDecimals(raw: string, decimals: number): string | null {
  if (!Number.isInteger(decimals) || decimals < 0 || decimals > 38) return null
  return multiplyNumericByTenPow(raw, -decimals)
}

export function parseAssetDecimals(raw: number | null | undefined): number | null {
  if (raw == null || !Number.isInteger(raw) || raw < 0 || raw > 38) return null
  return raw
}

export type ScaledPositionDisplay = {
  netPosition: string
  avgEntry: string
  costBasis: string
  realizedPnl: string
  realizedPnlUsd: number | null
}

/**
 * Per-row human display. JSON stays raw. Missing decimals → em-dash (do not assume 6).
 *
 * - Net position: quote (`asset_1`) human + symbol
 * - Cost basis / realized P&L: base (`asset_0`) human + symbol
 * - Avg entry: human base per 1 human quote (`raw × 10^(d1 − d0)`), labeled `BASE / QUOTE`
 */
export function formatScaledPosition(pos: IndexerPosition, oracle?: TraderOracleUsd): ScaledPositionDisplay {
  const d0 = parseAssetDecimals(pos.asset_0_decimals)
  const d1 = parseAssetDecimals(pos.asset_1_decimals)
  const base = pos.asset_0_symbol || 'base'
  const quote = pos.asset_1_symbol || 'quote'

  const netHuman = d1 == null ? null : scaleNumericByDecimals(pos.net_position_quote, d1)
  const costHuman = d0 == null ? null : scaleNumericByDecimals(pos.total_cost_base, d0)
  const pnlHuman = d0 == null ? null : scaleNumericByDecimals(pos.realized_pnl, d0)
  const avgHuman = d0 == null || d1 == null ? null : multiplyNumericByTenPow(pos.avg_entry_price, d1 - d0)

  return {
    netPosition: formatAmountWithSymbol(netHuman, quote),
    costBasis: formatAmountWithSymbol(costHuman, base),
    realizedPnl: formatPnlWithSymbol(pnlHuman, base),
    avgEntry: formatAvgEntry(avgHuman, base, quote),
    realizedPnlUsd: humanAmountToUsd(pnlHuman, pos.asset_0_symbol, pos.asset_0_denom, oracle),
  }
}

export type RealizedPnlUsdSummary = {
  /** Sum of priced rows only. `null` when none could be converted. */
  usd: number | null
  pricedPairs: number
  unpricedPairs: number
}

/** Cross-pair realized P&L in USD via P522-Q on the **base** token. Unpriced rows are omitted, not `$0`. */
export function sumRealizedPnlUsd(
  positions: IndexerPosition[] | undefined | null,
  oracle?: TraderOracleUsd
): RealizedPnlUsdSummary {
  if (positions == null) {
    return { usd: null, pricedPairs: 0, unpricedPairs: 0 }
  }
  if (positions.length === 0) {
    return { usd: 0, pricedPairs: 0, unpricedPairs: 0 }
  }
  let usd = 0
  let priced = 0
  let unpriced = 0
  for (const pos of positions) {
    const row = formatScaledPosition(pos, oracle)
    if (row.realizedPnlUsd == null) {
      unpriced += 1
    } else {
      usd += row.realizedPnlUsd
      priced += 1
    }
  }
  if (priced === 0) return { usd: null, pricedPairs: 0, unpricedPairs: unpriced }
  return { usd, pricedPairs: priced, unpricedPairs: unpriced }
}

export function formatSignedUsd(n: number | null | undefined): string {
  if (n == null || !Number.isFinite(n)) return TRADER_PNL_EM_DASH
  const abs = formatNum(Math.abs(n), 2)
  if (n > 0) return `+$${abs}`
  if (n < 0) return `-$${abs}`
  return `$${abs}`
}

function parseNumericParts(raw: string | null | undefined): { neg: boolean; combined: string; fracLen: number } | null {
  if (raw == null) return null
  const s = String(raw).trim()
  if (s === '') return null
  const neg = s.startsWith('-')
  const body = (neg ? s.slice(1) : s).replace(/^\+/, '')
  if (!/^(?:\d+(?:\.\d*)?|\.\d+)$/.test(body)) return null
  const [intRaw = '0', fracRaw = ''] = body.split('.')
  const intPart = intRaw.replace(/^0+/, '') || '0'
  const fracPart = fracRaw.replace(/0+$/, '')
  const combined = (intPart === '0' ? '' : intPart) + fracRaw
  const digits = combined.replace(/^0+/, '') || '0'
  return { neg, combined: digits, fracLen: fracPart.length === 0 && fracRaw.length === 0 ? 0 : fracRaw.length }
}

function formatAmountWithSymbol(human: string | null, symbol: string): string {
  if (human == null) return TRADER_PNL_EM_DASH
  return `${formatNum(human, 4)} ${symbol}`
}

function formatPnlWithSymbol(human: string | null, symbol: string): string {
  if (human == null) return TRADER_PNL_EM_DASH
  const n = Number(human)
  if (!Number.isFinite(n)) return TRADER_PNL_EM_DASH
  const prefix = n > 0 ? '+' : ''
  return `${prefix}${formatNum(human, 4)} ${symbol}`
}

function formatAvgEntry(human: string | null, base: string, quote: string): string {
  if (human == null) return TRADER_PNL_EM_DASH
  const n = Number(human)
  if (!Number.isFinite(n) || n === 0) return TRADER_PNL_EM_DASH
  const price = formatPairPrice(human, 6)
    .replace(/(\.\d*?)0+$/, '$1')
    .replace(/\.$/, '')
  return `${price} ${base} / ${quote}`
}

function humanAmountToUsd(
  human: string | null,
  symbol: string,
  denom: string | null | undefined,
  oracle?: TraderOracleUsd
): number | null {
  if (human == null) return null
  const n = Number(human)
  if (!Number.isFinite(n) || n === 0) {
    return n === 0 ? 0 : null
  }
  const per = quoteTokenUsd(classifyQuoteSymbol(symbol, denom), oracle?.ustcUsd, oracle?.luncUsd)
  if (per == null) return null
  const usd = n * per
  return Number.isFinite(usd) ? usd : null
}
