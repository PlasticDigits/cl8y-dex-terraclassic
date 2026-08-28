/**
 * /trade + /charts pair-orientation invert (GitLab #524 / #680).
 *
 * Display-only. Indexer `price` / `price_usd` / candles and on-chain
 * `place_limit_order` stay factory `token1` per `token0`. Convert at the UI edge
 * with {@link displayPriceToFactoryToken1PerToken0} before simulate/submit/gates.
 *
 * **Trade** defaults and storage: **T524-1–T524-11** (`cl8y-dex-trade-pair-invert:`).
 * **Charts** defaults and storage: **C680-1–C680-9** (`cl8y-dex-charts-pair-invert:`).
 * Do not write Charts orientation into the Trade key.
 *
 * See `docs/frontend.md` § Trade pair display invert / Charts UST1/USD hero
 * and `skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md` /
 * `skills/AGENTS_FRONTEND_CHARTS_UST1_HERO.md`.
 */

import type { ChartCandlePoint } from '@/components/charts/priceChartCandles'
import { LUNC_C_TOKEN_ADDRESS, UST1_TOKEN_ADDRESS, USTC_C_TOKEN_ADDRESS, VFDUSD_TOKEN_ADDRESS } from '@/utils/constants'
import {
  MAINNET_CUSTC_TOKEN_ADDRESS,
  MAINNET_UST1_TOKEN_ADDRESS,
  MAINNET_VFDUSD_TOKEN_ADDRESS,
} from '@/utils/ust1SecondaryMarket'

/** sessionStorage key prefix; value is `1` | `0` per factory pair address. Trade only. */
export const PAIR_DISPLAY_INVERT_STORAGE_PREFIX = 'cl8y-dex-trade-pair-invert:'

/** Charts-only invert persistence. Must not share the Trade prefix (**C680-7**). */
export const CHARTS_PAIR_DISPLAY_INVERT_STORAGE_PREFIX = 'cl8y-dex-charts-pair-invert:'

export type PairDisplayLeg = {
  symbol?: string | null
  contractAddr?: string | null
  contract_addr?: string | null
  denom?: string | null
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

function knownCustcContracts(): Set<string> {
  const set = new Set<string>([MAINNET_CUSTC_TOKEN_ADDRESS.toLowerCase()])
  const env = USTC_C_TOKEN_ADDRESS.trim().toLowerCase()
  if (env.startsWith('terra1')) set.add(env)
  return set
}

function knownCluncContracts(): Set<string> {
  const set = new Set<string>()
  const env = LUNC_C_TOKEN_ADDRESS.trim().toLowerCase()
  if (env.startsWith('terra1')) set.add(env)
  return set
}

function legSymbolUpper(leg: PairDisplayLeg | null | undefined): string {
  return (leg?.symbol ?? '').trim().toUpperCase()
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
 * Wrap cUSTC only (exact `CUSTC` / `cUSTC` or known wrap contract).
 * Native `uusd` / ticker `USTC` alone is not the hero wrap leg.
 */
export function isCustcLeg(leg: PairDisplayLeg | null | undefined): boolean {
  if (!leg) return false
  if ((leg.denom ?? '').trim().toLowerCase() === 'uusd') return false
  const addr = normalizeAddr(leg.contractAddr ?? leg.contract_addr)
  if (addr) {
    if (knownCustcContracts().has(addr)) return true
    if (knownUst1Contracts().has(addr)) return false
  }
  const upper = legSymbolUpper(leg)
  const raw = (leg.symbol ?? '').trim()
  return upper === 'CUSTC' || raw === 'cUSTC'
}

/** Wrap cLUNC only (exact `CLUNC` / `cLUNC` or known wrap contract). Native `uluna` is not enough. */
export function isCluncLeg(leg: PairDisplayLeg | null | undefined): boolean {
  if (!leg) return false
  if ((leg.denom ?? '').trim().toLowerCase() === 'uluna') return false
  const addr = normalizeAddr(leg.contractAddr ?? leg.contract_addr)
  if (addr) {
    if (knownCluncContracts().has(addr)) return true
    if (knownUst1Contracts().has(addr) || knownCustcContracts().has(addr)) return false
  }
  const upper = legSymbolUpper(leg)
  const raw = (leg.symbol ?? '').trim()
  return upper === 'CLUNC' || raw === 'cLUNC'
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

/**
 * Charts product default: factory orientation (UST1-as-base → UST1 USD).
 * Trade keeps {@link defaultDisplayInverted} (**T524-3** / **C543-3**).
 */
export function defaultChartsDisplayInverted(): boolean {
  return false
}

function readSessionFlag(prefix: string, pairAddr: string): boolean | null {
  if (typeof sessionStorage === 'undefined' || !pairAddr) return null
  try {
    const v = sessionStorage.getItem(`${prefix}${pairAddr}`)
    if (v === '1') return true
    if (v === '0') return false
    return null
  } catch {
    return null
  }
}

function writeSessionFlag(prefix: string, pairAddr: string, inverted: boolean): void {
  if (typeof sessionStorage === 'undefined' || !pairAddr) return
  try {
    sessionStorage.setItem(`${prefix}${pairAddr}`, inverted ? '1' : '0')
  } catch {
    // quota / private mode
  }
}

export function readChartsStoredPairDisplayInverted(pairAddr: string): boolean | null {
  return readSessionFlag(CHARTS_PAIR_DISPLAY_INVERT_STORAGE_PREFIX, pairAddr)
}

export function writeChartsStoredPairDisplayInverted(pairAddr: string, inverted: boolean): void {
  writeSessionFlag(CHARTS_PAIR_DISPLAY_INVERT_STORAGE_PREFIX, pairAddr, inverted)
}

function symbolsEqual(a: string, b: string): boolean {
  return a.trim().toUpperCase() === b.trim().toUpperCase()
}

function contractEquals(leg: PairDisplayLeg | null | undefined, raw: string): boolean {
  const addr = normalizeAddr(leg?.contractAddr ?? leg?.contract_addr)
  return !!addr && addr === raw.trim().toLowerCase()
}

/**
 * Match a `?price=` token onto a pair leg. Aliases: `USTC` → cUSTC, `LUNC` → cLUNC,
 * only when that wrap leg is on the pair.
 */
export function matchChartsPriceLeg(
  raw: string,
  asset0: PairDisplayLeg | null | undefined,
  asset1: PairDisplayLeg | null | undefined
): 'asset0' | 'asset1' | null {
  const trimmed = raw.trim()
  if (!trimmed) return null

  const tryLeg = (leg: PairDisplayLeg | null | undefined): boolean => {
    if (!leg) return false
    if (contractEquals(leg, trimmed)) return true
    if (symbolsEqual(leg.symbol ?? '', trimmed)) return true
    const upper = trimmed.toUpperCase()
    if (upper === 'USTC' && isCustcLeg(leg)) return true
    if (upper === 'LUNC' && isCluncLeg(leg)) return true
    return false
  }

  const on0 = tryLeg(asset0)
  const on1 = tryLeg(asset1)
  if (on0 && !on1) return 'asset0'
  if (on1 && !on0) return 'asset1'
  return null
}

/** Priced token is `asset_1` when inverted (display base = quote). */
export function chartsInvertedFromPriceMatch(match: 'asset0' | 'asset1'): boolean {
  return match === 'asset1'
}

export function chartsPriceTokenForInverted(
  inverted: boolean,
  asset0: PairDisplayLeg | null | undefined,
  asset1: PairDisplayLeg | null | undefined,
  token0Symbol: string,
  token1Symbol: string
): string {
  const { displayBase } = displayPairAssets(token0Symbol, token1Symbol, inverted)
  const preferred = displayBase.trim()
  if (preferred) return preferred
  const fallback = inverted ? (asset1?.symbol ?? '') : (asset0?.symbol ?? '')
  return fallback.trim() || (inverted ? 'Quote' : 'Base')
}

/**
 * Charts orientation: valid `?price=` → Charts session → Charts product default.
 * Never reads or writes the Trade invert key.
 */
export function resolveChartsDisplayInverted(pairAddr: string, priceMatch: 'asset0' | 'asset1' | null): boolean {
  if (priceMatch) return chartsInvertedFromPriceMatch(priceMatch)
  const stored = pairAddr ? readChartsStoredPairDisplayInverted(pairAddr) : null
  if (stored != null) return stored
  return defaultChartsDisplayInverted()
}
