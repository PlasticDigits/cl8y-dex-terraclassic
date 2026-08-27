-- GitLab #676: raw 18-decimal amounts overflow NUMERIC(38, 18) (|x| < 10^20).
-- ~100 human USTR / CL8Y is 10^20 raw — upsert fails after swap_events insert,
-- and replay skips the position update (swap already exists). Same class as
-- 20260817120000 (global_stats / pair volume) and candle human NUMERIC(78, 18).
--
-- Widen first so the rebuild below can store 6/18 and 18/6 raw inventories.
-- Replay is one-shot SQL matching position_tracker::update_position_on_swap.

ALTER TABLE trader_positions
    ALTER COLUMN net_position_quote TYPE NUMERIC(78, 18),
    ALTER COLUMN avg_entry_price TYPE NUMERIC(78, 18),
    ALTER COLUMN total_cost_base TYPE NUMERIC(78, 18),
    ALTER COLUMN realized_pnl TYPE NUMERIC(78, 18);

ALTER TABLE traders
    ALTER COLUMN total_realized_pnl TYPE NUMERIC(78, 18),
    ALTER COLUMN best_trade_pnl TYPE NUMERIC(78, 18),
    ALTER COLUMN worst_trade_pnl TYPE NUMERIC(78, 18),
    ALTER COLUMN total_fees_paid TYPE NUMERIC(78, 18);

DELETE FROM trader_positions;

DO $$
DECLARE
    rec RECORD;
    v_pos NUMERIC(78, 18);
    v_avg NUMERIC(78, 18);
    v_cost NUMERIC(78, 18);
    v_rpnl NUMERIC(78, 18);
    v_count INTEGER;
    v_exit NUMERIC(78, 18);
    v_pnl NUMERIC(78, 18);
    v_zero NUMERIC(78, 18) := 0;
BEGIN
    FOR rec IN
        SELECT
            se.sender,
            se.pair_id,
            p.asset_0_id,
            se.offer_asset_id,
            se.offer_amount,
            se.return_amount
        FROM swap_events se
        JOIN pairs p ON p.id = se.pair_id
        ORDER BY se.block_height, se.id
    LOOP
        SELECT
            net_position_quote, avg_entry_price, total_cost_base, realized_pnl, trade_count
        INTO v_pos, v_avg, v_cost, v_rpnl, v_count
        FROM trader_positions
        WHERE trader_address = rec.sender AND pair_id = rec.pair_id;

        IF NOT FOUND THEN
            v_pos := v_zero;
            v_avg := v_zero;
            v_cost := v_zero;
            v_rpnl := v_zero;
            v_count := 0;
        END IF;

        IF rec.offer_asset_id = rec.asset_0_id THEN
            v_pos := v_pos + rec.return_amount;
            v_cost := v_cost + rec.offer_amount;
            IF v_pos > v_zero THEN
                v_avg := v_cost / v_pos;
            ELSE
                v_avg := v_zero;
            END IF;
        ELSE
            IF rec.offer_amount > v_zero THEN
                v_exit := rec.return_amount / rec.offer_amount;
            ELSE
                v_exit := v_zero;
            END IF;
            v_pnl := (v_exit - v_avg) * rec.offer_amount;
            v_rpnl := v_rpnl + v_pnl;
            v_pos := v_pos - rec.offer_amount;
            IF v_pos < v_zero THEN
                v_pos := v_zero;
            END IF;
            IF v_pos > v_zero THEN
                v_cost := v_pos * v_avg;
            ELSE
                v_cost := v_zero;
                v_avg := v_zero;
            END IF;
        END IF;

        v_count := v_count + 1;

        INSERT INTO trader_positions
            (trader_address, pair_id, net_position_quote, avg_entry_price,
             total_cost_base, realized_pnl, trade_count)
        VALUES (rec.sender, rec.pair_id, v_pos, v_avg, v_cost, v_rpnl, v_count)
        ON CONFLICT (trader_address, pair_id)
          DO UPDATE SET
            net_position_quote = EXCLUDED.net_position_quote,
            avg_entry_price = EXCLUDED.avg_entry_price,
            total_cost_base = EXCLUDED.total_cost_base,
            realized_pnl = EXCLUDED.realized_pnl,
            trade_count = EXCLUDED.trade_count,
            updated_at = NOW();
    END LOOP;
END $$;
