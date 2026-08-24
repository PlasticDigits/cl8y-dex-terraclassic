-- GitLab #614: widen protocol fee source CHECKs for ust1-window mint/redeem.
-- GET paths still must not SUM protocol_fee_events. Pin is UST1_WINDOW_ADDRESS (not WRAP_MAPPER_ADDRESS).

ALTER TABLE protocol_fee_events
    DROP CONSTRAINT IF EXISTS protocol_fee_events_source_chk;
ALTER TABLE protocol_fee_events
    ADD CONSTRAINT protocol_fee_events_source_chk CHECK (
        source IN (
            'swap_amm',
            'book_take',
            'limit_place',
            'wrap',
            'unwrap',
            'ust1_mint',
            'ust1_redeem'
        )
    );

COMMENT ON COLUMN protocol_fee_events.source IS
    'swap_amm = pool commission_amount; book_take = limit_order_fills.commission_amount; limit_place = maker_fee_amount; wrap/unwrap = pinned WRAP_MAPPER_ADDRESS only; ust1_mint/ust1_redeem = pinned UST1_WINDOW_ADDRESS fee_amount only (never × fee_bps).';

ALTER TABLE protocol_fee_stats_by_source
    DROP CONSTRAINT IF EXISTS protocol_fee_stats_by_source_source_chk;
ALTER TABLE protocol_fee_stats_by_source
    ADD CONSTRAINT protocol_fee_stats_by_source_source_chk CHECK (
        source IN (
            'swap_amm',
            'book_take',
            'limit_place',
            'wrap',
            'unwrap',
            'ust1_mint',
            'ust1_redeem'
        )
    );

ALTER TABLE global_stats_24h
    ADD COLUMN IF NOT EXISTS ust1_window_configured BOOLEAN NOT NULL DEFAULT FALSE;

COMMENT ON COLUMN global_stats_24h.ust1_window_configured IS
    'True when UST1_WINDOW_ADDRESS parsed as a valid terra1 bech32 at last fee rollup. Unconfigured ust1_mint/ust1_redeem omitted from breakdown (not fake $0).';
