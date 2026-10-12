import type { IndexerRouteSolveResponse } from '@/types'

export const UNAVAILABLE_ECONOMIC_EXPLANATION = 'Cost breakdown unavailable.'
const COMPONENTS = ['wrapper_loss', 'trading_cost', 'market_movement', 'reference_divergence'] as const
const RAW_MAX = (1n << 128n) - 1n

function record(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value) ? (value as Record<string, unknown>) : null
}
function exactKeys(value: Record<string, unknown>, keys: readonly string[]): boolean {
  return Object.keys(value).length === keys.length && Object.keys(value).every((key) => keys.includes(key))
}
function signedRaw(value: unknown): bigint | null {
  if (typeof value !== 'string' || !/^(?:0|-?[1-9]\d{0,38})$/.test(value)) return null
  const n = BigInt(value)
  return n <= RAW_MAX && n >= -RAW_MAX ? n : null
}

/** Advisory only: allowlisted copy after bounded reconciliation, never an execution permission. */
export function routeEconomicExplanation(quote: IndexerRouteSolveResponse): string {
  const attribution = record(quote.economic_attribution)
  const metadata = quote.execution_metadata
  const now = Date.now()
  if (
    !metadata ||
    !Number.isSafeInteger(metadata.observed_at_ms) ||
    !Number.isSafeInteger(metadata.expires_at_ms) ||
    metadata.observed_at_ms < 0 ||
    metadata.observed_at_ms > now ||
    metadata.expires_at_ms <= now ||
    metadata.expires_at_ms <= metadata.observed_at_ms ||
    metadata.expires_at_ms - metadata.observed_at_ms > 20000 ||
    (quote.estimated_amount_out_net != null && quote.estimated_amount_out_net !== quote.estimated_amount_out)
  )
    return UNAVAILABLE_ECONOMIC_EXPLANATION

  if (
    !attribution ||
    !metadata ||
    !exactKeys(attribution, [
      'version',
      'status',
      'method',
      'generation',
      'observed_at_ms',
      'expires_at_ms',
      'fair_amount_out',
      'net_amount_out',
      'components',
      'dominant_cause',
    ]) ||
    attribution.version !== 1 ||
    attribution.status !== 'available' ||
    attribution.method !== 'spot_bridge_v1' ||
    attribution.generation !== metadata.generation ||
    attribution.observed_at_ms !== metadata.observed_at_ms ||
    attribution.expires_at_ms !== metadata.expires_at_ms ||
    attribution.fair_amount_out !== quote.spot_amount_out ||
    attribution.net_amount_out !== quote.estimated_amount_out
  )
    return UNAVAILABLE_ECONOMIC_EXPLANATION
  const fair = signedRaw(attribution.fair_amount_out),
    net = signedRaw(attribution.net_amount_out)
  const components = record(attribution.components)
  if (fair == null || net == null || fair <= 0n || net <= 0n || !components || !exactKeys(components, COMPONENTS))
    return UNAVAILABLE_ECONOMIC_EXPLANATION
  const values = COMPONENTS.map((key) => signedRaw(components[key]))
  if (values.some((value) => value == null)) return UNAVAILABLE_ECONOMIC_EXPLANATION
  const amounts = values as bigint[]
  if (amounts.reduce((sum, value) => sum + value, 0n) !== fair - net) return UNAVAILABLE_ECONOMIC_EXPLANATION
  const abs = amounts.map((n) => (n < 0n ? -n : n))
  const largest = abs.reduce((a, b) => (a > b ? a : b), 0n)
  const matches = abs.map((n, i) => (n === largest ? i : -1)).filter((i) => i >= 0)
  const cause = matches.length === 1 ? COMPONENTS[matches[0]] : 'mixed'
  if (attribution.dominant_cause !== cause) return UNAVAILABLE_ECONOMIC_EXPLANATION
  if (amounts.every((amount) => amount === 0n)) return 'No difference from the fair reference.'
  switch (cause) {
    case 'wrapper_loss':
      return amounts[0] >= 0n ? 'Wrapping costs contribute most.' : UNAVAILABLE_ECONOMIC_EXPLANATION
    case 'trading_cost':
      return amounts[1] >= 0n ? 'Trading fees, taxes and hooks contribute most.' : UNAVAILABLE_ECONOMIC_EXPLANATION
    case 'market_movement':
      return amounts[2] >= 0n
        ? 'Trade size and available liquidity contribute most.'
        : 'Pool or book execution improves the output most.'
    case 'reference_divergence':
      return 'Reference and pool prices differ most.'
    default:
      return 'Several costs and price differences contribute.'
  }
}
