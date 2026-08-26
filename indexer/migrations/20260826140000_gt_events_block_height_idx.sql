-- GeckoTerminal /gt/events polls inclusive block ranges every ~2s (#646).
CREATE INDEX IF NOT EXISTS idx_swap_events_block_height ON swap_events (block_height);
CREATE INDEX IF NOT EXISTS idx_liquidity_events_block_height ON liquidity_events (block_height);
