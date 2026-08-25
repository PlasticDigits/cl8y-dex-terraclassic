'use strict'

const { INDEXER_DAILY_URL, ADAPTER_START } = require('../gems')

function dailyUrl(timestamp, host = INDEXER_DAILY_URL) {
  const ts = Number(timestamp)
  if (!Number.isInteger(ts) || ts < 0 || ts % 86400 !== 0) {
    throw new Error('timestamp must be unix 00:00 UTC')
  }
  const base = host.split('?')[0]
  return `${base}?timestamp=${ts}`
}

function asNumberOrNull(value) {
  if (value == null) return null
  if (value === '0') return 0
  const n = Number(value)
  return Number.isFinite(n) ? n : null
}

function mapVolume(json) {
  return { dailyVolume: asNumberOrNull(json.volume_usd) }
}

function mapAsset(json, ticker) {
  const row = (json.assets && json.assets[ticker]) || {}
  return {
    ticker,
    volume: asNumberOrNull(row.volume_usd),
    fees: asNumberOrNull(row.fees_usd),
    price: asNumberOrNull(row.price_usd),
    circulating: asNumberOrNull(row.circulating),
    product: row.product || null,
    pegType: row.peg_type ?? null,
  }
}

function mapFees(json) {
  const fees = json.fees || {}
  return {
    dailyFees: asNumberOrNull(json.daily_fees_usd),
    dailyRevenue: asNumberOrNull(json.daily_revenue_usd),
    dailyProtocolRevenue: asNumberOrNull(json.daily_protocol_revenue_usd),
    dailySupplySideRevenue: 0,
    breakdown: {
      swap_amm: asNumberOrNull(fees.swap_amm),
      book_take: asNumberOrNull(fees.book_take),
      limit_place: asNumberOrNull(fees.limit_place),
      wrap: asNumberOrNull(fees.wrap),
      unwrap: asNumberOrNull(fees.unwrap),
      ust1_mint: asNumberOrNull(fees.ust1_mint),
      ust1_redeem: asNumberOrNull(fees.ust1_redeem),
    },
  }
}

const METHODOLOGY = {
  Volume:
    'UTC calendar-day SUM(swap_events.volume_usd) once per taker swap. Excludes columbus-5 gem pairs, wrap/unwrap, UST1 window, and limit_order_fills (hybrid L10).',
  Fees:
    'Treasury-bound PFee/L7: swap_amm + book_take + limit_place plus labeled wrap/window. spread_amount and community-tax extra-debit are not fees.',
  Revenue: 'Same as Fees — protocol keeps pair treasury commission (CMM).',
  ProtocolRevenue: 'Same as Fees.',
  SupplySideRevenue: '0 — LPs earn inventory/spread, not a transferred commission.',
}

const BREAKDOWN_METHODOLOGY = {
  swap_amm: 'Pair pool commission_amount to FEE_CONFIG.treasury',
  book_take: 'limit_order_fills.commission_amount only (not swap book_commission_amount)',
  limit_place: 'Maker placement fee to treasury',
  wrap: 'Pinned wrap-mapper treasury fee (labeled)',
  unwrap: 'Pinned wrap-mapper unwrap fee (labeled)',
  ust1_mint: 'Pinned ust1-window mint fee (labeled)',
  ust1_redeem: 'Pinned ust1-window redeem fee (labeled)',
}

module.exports = {
  ADAPTER_START,
  INDEXER_DAILY_URL,
  dailyUrl,
  mapVolume,
  mapAsset,
  mapFees,
  METHODOLOGY,
  BREAKDOWN_METHODOLOGY,
}
