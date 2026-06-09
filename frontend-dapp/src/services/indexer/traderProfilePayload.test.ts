import { describe, expect, it } from 'vitest'
import { parseIndexerTraderPayload } from './traderProfilePayload'

describe('parseIndexerTraderPayload', () => {
  const minimal = {
    address: 'terra1abcdefghijklmnopqrstuvwxyz1234567890abcd',
    total_trades: 3,
    total_volume: '1',
    volume_24h: '0',
    volume_7d: '0',
    volume_30d: '0',
    tier_id: null,
    tier_name: null,
    registered: false,
    first_trade_at: null,
    last_trade_at: null,
    total_realized_pnl: '0',
    best_trade_pnl: '0',
    worst_trade_pnl: '0',
    total_fees_paid: '0',
  }

  it('accepts a well-formed indexer object', () => {
    const t = parseIndexerTraderPayload(minimal)
    expect(t.address).toBe(minimal.address)
    expect(t.total_trades).toBe(3)
  })

  it('rejects null, arrays, and non-objects', () => {
    expect(() => parseIndexerTraderPayload(null)).toThrow(/invalid trader profile/i)
    expect(() => parseIndexerTraderPayload([])).toThrow(/invalid trader profile/i)
    expect(() => parseIndexerTraderPayload('x')).toThrow(/invalid trader profile/i)
  })

  it('rejects objects without a valid terra address', () => {
    expect(() => parseIndexerTraderPayload({})).toThrow(/invalid trader profile/i)
    expect(() => parseIndexerTraderPayload({ address: 'not-terra' })).toThrow(/invalid trader profile/i)
  })

  it('coerces numeric strings and fills numeric defaults', () => {
    const t = parseIndexerTraderPayload({
      ...minimal,
      total_trades: '12',
      total_volume: 99,
    })
    expect(t.total_trades).toBe(12)
    expect(t.total_volume).toBe('99')
  })

  it('preserves null best/worst trade pnl (#344)', () => {
    const t = parseIndexerTraderPayload({
      ...minimal,
      best_trade_pnl: null,
      worst_trade_pnl: null,
    })
    expect(t.best_trade_pnl).toBe(null)
    expect(t.worst_trade_pnl).toBe(null)
  })

  it('maps tier_id to null when invalid', () => {
    const t = parseIndexerTraderPayload({
      ...minimal,
      tier_id: 'nope',
    })
    expect(t.tier_id).toBe(null)
  })
})
