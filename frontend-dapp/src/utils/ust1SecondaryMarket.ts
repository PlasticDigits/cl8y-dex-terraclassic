/**
 * UST1 secondary AMM market helpers (GitLab #508 / parent #502).
 *
 * Invariant U1: AMM is secondary price discovery only. Oracle mint/redeem stays on `/ust1`
 * (ust1-window). Never describe Trade/Swap as mint or redeem.
 */

/** columbus-5 anchors from issue #508 — override via Vite env when present. */
export const MAINNET_UST1_TOKEN_ADDRESS = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
export const MAINNET_VFDUSD_TOKEN_ADDRESS = 'terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3'
export const MAINNET_CUSTC_TOKEN_ADDRESS = 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch'

export type Ust1SecondaryQuoteLeg = 'vFDUSD' | 'cUSTC'

export type Ust1SecondaryMarketTokens = {
  ust1: string
  quote: string
  quoteLeg: Ust1SecondaryQuoteLeg
}

/**
 * Short retail copy for linking Trade/Swap from `/ust1` (cognitive-load friendly).
 * Must not say mint/redeem for the AMM path.
 */
export const UST1_SECONDARY_MARKET_BLURB =
  'Need a secondary market? Use Trade or Swap — not the same as mint or redeem here.'

/** Create-pair page: AMM markets ≠ oracle window mint/redeem (U1). */
export const UST1_CREATE_PAIR_SECONDARY_NOTICE =
  'Creating a pair opens an AMM market only. UST1 mint and redeem stay on the /ust1 oracle window — not here.'

/** Optional: set after Path A create so UI can deep-link Trade without waiting on discovery. */
export const UST1_SECONDARY_PAIR_ADDRESS = import.meta.env.VITE_UST1_SECONDARY_PAIR_ADDRESS || ''

export function isUst1SecondaryPairConfigured(): boolean {
  return UST1_SECONDARY_PAIR_ADDRESS.trim().length > 0
}

/** Forbidden phrases that would market AMM as the oracle path (U1). */
export const UST1_AMM_AS_MINT_FORBIDDEN = [
  'mint on the amm',
  'mint via swap',
  'redeem on the amm',
  'amm mint',
  'swap to mint ust1',
] as const

export function resolvesUst1SecondaryTokens(
  ust1Env: string,
  vfdusdEnv: string,
  quoteLeg: Ust1SecondaryQuoteLeg = 'vFDUSD',
  custcEnv = ''
): Ust1SecondaryMarketTokens | null {
  const ust1 = ust1Env.trim() || MAINNET_UST1_TOKEN_ADDRESS
  const quote =
    quoteLeg === 'cUSTC'
      ? custcEnv.trim() || MAINNET_CUSTC_TOKEN_ADDRESS
      : vfdusdEnv.trim() || MAINNET_VFDUSD_TOKEN_ADDRESS
  if (!ust1.startsWith('terra1') || !quote.startsWith('terra1')) return null
  return { ust1, quote, quoteLeg }
}

/**
 * Trade deep-link for a UST1 secondary pair.
 * Matches app routes: `/trade` or `/trade/:pairAddr` (see `tradePairRoute.ts`).
 * Prefer an explicit pair address, then `VITE_UST1_SECONDARY_PAIR_ADDRESS`.
 */
export function ust1SecondaryTradePath(pairAddress?: string): string {
  const pair = (pairAddress ?? UST1_SECONDARY_PAIR_ADDRESS).trim()
  if (!pair) return '/trade'
  return `/trade/${pair}`
}

/**
 * Swap CTA target for `/ust1` secondary-market links.
 * SwapPage does not yet honor token query params — keep the home Swap route only.
 */
export function ust1SecondarySwapPath(): string {
  return '/'
}

/** True when copy would violate U1 (case-insensitive substring match). */
export function copyImpliesAmmIsMintRedeem(text: string): boolean {
  const lower = text.toLowerCase()
  return UST1_AMM_AS_MINT_FORBIDDEN.some((p) => lower.includes(p))
}

export function assertSecondaryMarketCopy(text: string): void {
  if (copyImpliesAmmIsMintRedeem(text)) {
    throw new Error(`UST1 secondary market copy violates U1 (#508): ${text}`)
  }
}
