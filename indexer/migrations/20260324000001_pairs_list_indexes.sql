-- Indexes for paginated / sorted pair listing
CREATE INDEX IF NOT EXISTS idx_pairs_fee_bps ON pairs (fee_bps);
CREATE INDEX IF NOT EXISTS idx_pairs_created_block ON pairs (created_at_block DESC NULLS LAST);
CREATE INDEX IF NOT EXISTS idx_assets_symbol_lower ON assets (LOWER(symbol));
