-- #1209/#1210/#1354: additive pinned native creation and UST1 invoice sources.
-- Preserve #1269's two partial replay indexes; all these sources have NULL pair_id.
ALTER TABLE protocol_fee_events DROP CONSTRAINT protocol_fee_events_source_chk;
ALTER TABLE protocol_fee_events ADD CONSTRAINT protocol_fee_events_source_chk CHECK
    (source IN ('swap_amm','book_take','limit_place','wrap','unwrap','ust1_mint','ust1_redeem','pair_creation','sku_unlock','settings_fee'));
ALTER TABLE protocol_fee_stats_by_source DROP CONSTRAINT protocol_fee_stats_by_source_source_chk;
ALTER TABLE protocol_fee_stats_by_source ADD CONSTRAINT protocol_fee_stats_by_source_source_chk CHECK
    (source IN ('swap_amm','book_take','limit_place','wrap','unwrap','ust1_mint','ust1_redeem','pair_creation','sku_unlock','settings_fee'));
ALTER TABLE defillama_daily_fees DROP CONSTRAINT defillama_daily_fees_source_check;
ALTER TABLE defillama_daily_fees ADD CONSTRAINT defillama_daily_fees_source_check CHECK
    (source IN ('swap_amm','book_take','limit_place','wrap','unwrap','ust1_mint','ust1_redeem','pair_creation','sku_unlock','settings_fee'));
ALTER TABLE global_stats_24h
    ADD COLUMN factory_configured BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN community_invoices_configured BOOLEAN NOT NULL DEFAULT FALSE;
COMMENT ON COLUMN global_stats_24h.factory_configured IS 'Pinned FACTORY_ADDRESS enables pair_creation; absent pin omits the source, configured idle is zero. #1209';
COMMENT ON COLUMN global_stats_24h.community_invoices_configured IS 'Pinned COMMUNITY_TOKEN_LAUNCHER enables sku_unlock/settings_fee; each fee additionally requires exact UST1 config identity and trustworthy emitter provenance. #1210';
