import type { IndexerRouteExecutionMetadata } from '@/types'

const SCALE = 10n ** 18n
const UINT128_MAX = (1n << 128n) - 1n

function raw(value: unknown): bigint | null {
  return typeof value === 'string' && /^(?:0|[1-9]\d{0,38})$/.test(value) && BigInt(value) <= UINT128_MAX
    ? BigInt(value)
    : null
}

function tolerance(value: unknown): bigint | null {
  if (typeof value !== 'string' || !/^0(?:\.\d{1,18})?$/.test(value)) return null
  const amount = BigInt((value.split('.')[1] ?? '').padEnd(18, '0'))
  return amount > 0n && amount <= SCALE / 2n ? amount : null
}

function decimal(value: bigint): string {
  if (value === SCALE) return '1'
  return `0.${value.toString().padStart(18, '0')}`.replace(/0+$/, '').replace(/\.$/, '')
}

/** Independently reproduce pinned L9 economics and the quote-relative allowance.
 * UI protection always anchors net floors; the market ceiling only permits the quoted baseline. */
export function deriveQuoteRelativeMarketSpread(metadata: IndexerRouteExecutionMetadata): string | null {
  const selected = tolerance(metadata.max_spread)
  if (selected == null || !Array.isArray(metadata.hops) || !metadata.hops.length || metadata.hops.length > 4)
    return null
  let ceiling = 0n
  for (const hop of metadata.hops) {
    if (!hop) return null
    const pool = raw(hop.pool_return_amount),
      fee = raw(hop.commission_amount),
      book = raw(hop.book_return_net),
      poolInput = raw(hop.pool_input_actual),
      consumed = raw(hop.book_offer_consumed),
      spread = raw(hop.spread_amount)
    if (pool == null || fee == null || book == null || poolInput == null || consumed == null || spread == null)
      return null
    const poolGross = pool + fee,
      denominator = poolGross + book
    const expectedBook = poolInput > 0n && consumed > 0n ? (pool * consumed) / poolInput : 0n
    const shortfall = expectedBook > book ? expectedBook - book : 0n
    const numerator = (spread < poolGross ? spread : poolGross) + shortfall
    if (denominator <= 0n || denominator > UINT128_MAX || numerator > denominator || numerator > UINT128_MAX)
      return null
    const scaledNumerator = (numerator * SCALE + denominator * selected) * SCALE
    const scaledDenominator = denominator * (SCALE - selected)
    const relative = (scaledNumerator + scaledDenominator - 1n) / scaledDenominator
    const hopCeiling = relative > SCALE ? SCALE : relative
    if (hopCeiling > ceiling) ceiling = hopCeiling
  }
  return decimal(ceiling)
}

export function quoteRelativeNetFloor(output: unknown, selectedSpread: string): string | null {
  const amount = raw(output),
    selected = tolerance(selectedSpread)
  if (amount == null || amount <= 0n || selected == null) return null
  const floor = (amount * (SCALE - selected) + SCALE - 1n) / SCALE
  return (floor > 0n ? floor : 1n).toString()
}

/** Unknown policies and modified ceilings fail closed; legacy quotes retain their original L9 guard. */
export function validatedLocalMarketSpread(metadata: IndexerRouteExecutionMetadata): string | null {
  if (metadata.protection_policy == null) {
    return metadata.execution_market_spread == null ? metadata.max_spread : null
  }
  if (metadata.protection_policy !== 'quote_relative_v1') return null
  if (tolerance(metadata.max_spread) == null) return null
  const minimum = raw(metadata.minimum_receive)
  if (minimum == null || minimum <= 0n) return null
  if (metadata.policy === 'wrapper_only') return metadata.execution_market_spread == null ? metadata.max_spread : null
  if (
    !Array.isArray(metadata.hops) ||
    metadata.hops.some((hop) => {
      const output = raw(hop?.return_amount),
        floor = raw(hop?.min_return)
      return output == null || output <= 0n || floor == null || floor <= 0n
    })
  )
    return null
  const expected = deriveQuoteRelativeMarketSpread(metadata)
  return expected != null && metadata.execution_market_spread === expected ? expected : null
}
