/**
 * /trade + /charts pair-orientation invert (GitLab #524).
 *
 * Display-only. Indexer `price` / `price_usd` / candles and on-chain
 * `place_limit_order` stay factory `token1` per `token0`. Convert at the UI edge
 * with {@link displayPriceToFactoryToken1PerToken0} before simulate/submit/gates.
 *
 * Invariants **T524-1–T524-11** — see `docs/frontend.md` § Trade pair display invert
 * and `skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`.
 */

import type { ChartCandlePoint } from '@/components/charts/priceChartCandles'
import { LUNC_C_TOKEN_ADDRESS, UST1_TOKEN_ADDRESS, USTC_C_TOKEN_ADDRESS, VFDUSD_TOKEN_ADDRESS } from '@/utils/constants'
import {
  MAINNET_CUSTC_TOKEN_ADDRESS,
  MAINNET_UST1_TOKEN_ADDRESS,
  MAINNET_VFDUSD_TOKEN_ADDRESS,
} from '@/utils/ust1SecondaryMarket'

/** sessionStorage key prefix; value is `1` | `0` per factory pair address. */
export const PAIR_DISPLAY_INVERT_STORAGE_PREFIX = 'cl8y-dex-trade-pair-invert:'

export type PairDisplayLeg = {
  symbol?: string | null
  contractAddr?: string | null
  contract_addr?: string | null
}

export type FactorySide = 'bid' | 'ask'

function normalizeAddr(raw?: string | null): string {
  return (raw ?? '').trim().toLowerCase()
}

function knownUst1Contracts(): Set<string> {
  const set = new Set<string>([MAINNET_UST1_TOKEN_ADDRESS.toLowerCase()])
  const env = UST1_TOKEN_ADDRESS.trim().toLowerCase()
  if (env.startsWith('terra1')) set.add(env)
  return set
}

function knownNonUst1Contracts(): Set<string> {
  const set = new Set<string>([MAINNET_CUSTC_TOKEN_ADDRESS.toLowerCase(), MAINNET_VFDUSD_TOKEN_ADDRESS.toLowerCase()])
  for (const addr of [USTC_C_TOKEN_ADDRESS, LUNC_C_TOKEN_ADDRESS, VFDUSD_TOKEN_ADDRESS]) {
    const n = addr.trim().toLowerCase()
    if (n.startsWith('terra1')) set.add(n)
  }
  return set
}

export function parseFinitePositive(raw: string | number | null | undefined): number | null {
  if (raw == null) return null
  const n = typeof raw === 'number' ? raw : parseFloat(String(raw).trim())
  if (!Number.isFinite(n) || n <= 0) return null
  return n
}

/** Exact `UST1` after trim/casefold, or a known UST1 contract. Never substring-match `cUSTC`. */
export function isUst1Leg(leg: PairDisplayLeg | null | undefined): boolean {
  if (!leg) return false
  const addr = normalizeAddr(leg.contractAddr ?? leg.contract_addr)
  if (addr) {
    if (knownUst1Contracts().has(addr)) return true
    if (knownNonUst1Contracts().has(addr)) return false
  }
  return (leg.symbol ?? '').trim().toUpperCase() === 'UST1'
}

/**
 * Default invert when factory base is UST1 and the other leg is not.
 * UST1 already as `asset_1` stays factory-oriented (other token is already base).
 */
export function defaultDisplayInverted(
  asset0: PairDisplayLeg | null | undefined,
  asset1: PairDisplayLeg | null | undefined
): boolean {
  return isUst1Leg(asset0) && !isUst1Leg(asset1)
}

export function displayPairAssets(
  token0Symbol: string,
  token1Symbol: string,
  inverted: boolean
): { displayBase: string; displayQuote: string } {
  if (inverted) return { displayBase: token1Symbol, displayQuote: token0Symbol }
  return { displayBase: token0Symbol, displayQuote: token1Symbol }
}

/** Compact pill label `BASE/QUOTE` (no spaces) for chart + ticket chrome. */
export function pairDisplayPillLabel(displayBase: string, displayQuote: string): string {
  return `${displayBase}/${displayQuote}`
}

/** Screen-reader name for the invert control (names both symbols). */
export function pairDisplayInvertAriaLabel(displayBase: string, displayQuote: string): string {
  return `Show ${displayQuote} / ${displayBase} pricing`
}

export function formatInvertedDecimal(n: number): string | null {
  if (!Number.isFinite(n) || n <= 0) return null
  const s = n >= 1e-12 && n < 1e15 ? n.toFixed(12) : n.toPrecision(12)
  const trimmed = s.replace(/(\.\d*?)0+$/, '$1').replace(/\.$/, '')
  const parsed = parseFloat(trimmed)
  if (!Number.isFinite(parsed) || parsed <= 0) return null
  return trimmed
}

export function invertFinitePositive(raw: string | number | null | undefined): number | null {
  const n = parseFinitePositive(raw)
  if (n == null) return null
  const inv = 1 / n
  if (!Number.isFinite(inv) || inv <= 0) return null
  return inv
}

/**
 * USD of 1 display base when inverted: `factoryUsd / humanQuotePerBase`.
 * Drops non-finite / non-positive inputs (never `Infinity` / `NaN`).
 *
 * Use this for **Price (USD)** (headline + candles). {@link invertOhlc} is **only**
 * for human quote-per-base book/limit prices — never `1/x` a USD-of-asset_0 series.
 */
export function invertUsd(
  factoryUsd: string | number | null | undefined,
  humanQuotePerBase?: string | number | null
): string | null {
  const n = invertUsdNumber(factoryUsd, humanQuotePerBase)
  return n == null ? null : formatInvertedDecimal(n)
}

/** Numeric `invertUsd` for per-bar candle OHLC (GitLab #543). */
export function invertUsdNumber(
  factoryUsd: string | number | null | undefined,
  humanQuotePerBase?: string | number | null
): number | null {
  const usd = parseFinitePositive(factoryUsd)
  const px = parseFinitePositive(humanQuotePerBase)
  if (usd == null || px == null) return null
  const out = usd / px
  if (!Number.isFinite(out) || out <= 0) return null
  return out
}

/** Reciprocal OHLC with high/low swap. Drops non-finite / non-positive reciprocals. */
export function invertOhlc(points: ChartCandlePoint[]): ChartCandlePoint[] {
  const out: ChartCandlePoint[] = []
  for (const p of points) {
    const open = invertFinitePositive(p.open)
    const close = invertFinitePositive(p.close)
    const high = invertFinitePositive(p.high)
    const low = invertFinitePositive(p.low)
    if (open == null || close == null || high == null || low == null) continue
    out.push({ time: p.time, open, high: low, low: high, close })
  }
  return out
}

/** Display quote-per-display-base → factory `token1` per `token0`. */
export function displayPriceToFactoryToken1PerToken0(
  displayPrice: string | number | null | undefined,
  inverted: boolean
): string | null {
  const n = parseFinitePositive(displayPrice)
  if (n == null) return null
  if (!inverted) return formatInvertedDecimal(n) ?? String(n)
  return formatInvertedDecimal(1 / n)
}

/** Factory `token1` per `token0` → display quote-per-display-base. */
export function factoryToken1PerToken0ToDisplayPrice(
  factoryPrice: string | number | null | undefined,
  inverted: boolean
): string | null {
  return displayPriceToFactoryToken1PerToken0(factoryPrice, inverted)
}

/**
 * Display Buy/Sell of the displayed base → factory bid/ask of token0.
 * Inverted Buy {other} = factory ask (sell token0 / buy quote).
 */
export function factorySideFromDisplay(displaySide: FactorySide, inverted: boolean): FactorySide {
  if (!inverted) return displaySide
  return displaySide === 'bid' ? 'ask' : 'bid'
}

/** Factory bid/ask → display Buy/Sell of the displayed base. */
export function displaySideFromFactory(factorySide: FactorySide, inverted: boolean): FactorySide {
  return factorySideFromDisplay(factorySide, inverted)
}

export function readStoredPairDisplayInverted(pairAddr: string): boolean | null {
  if (typeof sessionStorage === 'undefined') return null
  const key = `${PAIR_DISPLAY_INVERT_STORAGE_PREFIX}${pairAddr}`
  try {
    const v = sessionStorage.getItem(key)
    if (v === '1') return true
    if (v === '0') return false
    return null
  } catch {
    return null
  }
}

export function writeStoredPairDisplayInverted(pairAddr: string, inverted: boolean): void {
  if (typeof sessionStorage === 'undefined' || !pairAddr) return
  try {
    sessionStorage.setItem(`${PAIR_DISPLAY_INVERT_STORAGE_PREFIX}${pairAddr}`, inverted ? '1' : '0')
  } catch {
    // quota / private mode
  }
}

export function resolvePairDisplayInverted(
  pairAddr: string,
  asset0: PairDisplayLeg | null | undefined,
  asset1: PairDisplayLeg | null | undefined
): boolean {
  const stored = pairAddr ? readStoredPairDisplayInverted(pairAddr) : null
  if (stored != null) return stored
  return defaultDisplayInverted(asset0, asset1)
}
