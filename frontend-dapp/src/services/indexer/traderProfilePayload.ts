import type { IndexerTrader } from '@/types'

function isRecord(x: unknown): x is Record<string, unknown> {
  return x !== null && typeof x === 'object' && !Array.isArray(x)
}

function asFiniteNumber(x: unknown, fallback: number): number {
  if (typeof x === 'number' && Number.isFinite(x)) return x
  if (typeof x === 'string' && x.trim() !== '') {
    const n = Number(x)
    if (Number.isFinite(n)) return n
  }
  return fallback
}

function asString(x: unknown, fallback = '0'): string {
  if (typeof x === 'string') return x
  if (x === null || x === undefined) return fallback
  return String(x)
}

function asNullableString(x: unknown): string | null {
  if (x === null || x === undefined) return null
  if (typeof x === 'string') return x
  return String(x)
}

/**
 * Normalizes indexer `GET /api/v1/traders/:address` JSON into {@link IndexerTrader}.
 * Throws if the payload is not a usable object (wrong shape, array body, missing address).
 *
 * Invariant: the Trader Profile page must never render arbitrary JSON as a trader — malformed
 * responses surface as query errors instead of crashing the route tree (GitLab #126).
 */
export function parseIndexerTraderPayload(data: unknown): IndexerTrader {
  if (!isRecord(data)) {
    throw new Error('Indexer returned an invalid trader profile')
  }

  const address = data.address
  if (typeof address !== 'string' || address.length < 40) {
    throw new Error('Indexer returned an invalid trader profile')
  }

  return {
    address,
    total_trades: asFiniteNumber(data.total_trades, 0),
    total_volume: asString(data.total_volume, '0'),
    volume_24h: asString(data.volume_24h, '0'),
    volume_7d: asString(data.volume_7d, '0'),
    volume_30d: asString(data.volume_30d, '0'),
    tier_id: (() => {
      if (data.tier_id === null || data.tier_id === undefined) return null
      if (typeof data.tier_id === 'number' && Number.isFinite(data.tier_id)) return data.tier_id
      if (typeof data.tier_id === 'string' && data.tier_id.trim() !== '') {
        const n = Number(data.tier_id)
        if (Number.isFinite(n)) return n
      }
      return null
    })(),
    tier_name: data.tier_name === null || data.tier_name === undefined ? null : asNullableString(data.tier_name),
    registered: Boolean(data.registered),
    first_trade_at:
      data.first_trade_at === null || data.first_trade_at === undefined ? null : asNullableString(data.first_trade_at),
    last_trade_at:
      data.last_trade_at === null || data.last_trade_at === undefined ? null : asNullableString(data.last_trade_at),
    total_realized_pnl: asString(data.total_realized_pnl, '0'),
    best_trade_pnl: asString(data.best_trade_pnl, '0'),
    worst_trade_pnl: asString(data.worst_trade_pnl, '0'),
    total_fees_paid: asString(data.total_fees_paid, '0'),
  }
}
