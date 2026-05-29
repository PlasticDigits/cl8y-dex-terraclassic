/**
 * Must match `indexer/scripts/seed-charts-integration.sql` pair row.
 * Candle rows must stay inside the indexer default 90-day API window — see `docs/indexer-invariants.md`.
 */
export const CHARTS_INTEGRATION_PAIR_ADDRESS = 'terra1paircontractabc'
