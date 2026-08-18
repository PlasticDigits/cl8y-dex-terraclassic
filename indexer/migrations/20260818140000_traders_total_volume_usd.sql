-- GitLab #553: restore traders.total_volume_usd as P522-Q USD (not mixed-unit raw).
-- Historical: 20260310000002 renamed the original total_volume_usd column to total_volume
-- (raw SUM(offer_amount)). Retail Charts leaderboard / trader profile must not format that.
--
-- NULL = no priced swaps for this sender. JSON maps NULL + trades > 0 to null (UI —).
-- Do not invent a second USD formula — SUM(swap_events.volume_usd) from volume_usd_for_swap.

ALTER TABLE traders
    ADD COLUMN IF NOT EXISTS total_volume_usd NUMERIC(38, 18);

COMMENT ON COLUMN traders.total_volume_usd IS
    'SUM(swap_events.volume_usd) for this sender (P522-Q catalog, GitLab #553). NULL when no priced swaps. total_volume stays raw offer_amount.';

CREATE INDEX IF NOT EXISTS idx_traders_total_volume_usd
    ON traders (total_volume_usd DESC NULLS LAST);

UPDATE traders t
SET total_volume_usd = sub.usd
FROM (
    SELECT
        sender,
        LEAST(
            SUM(volume_usd),
            POWER(10::numeric, 20) - POWER(10::numeric, -18)
        ) AS usd
    FROM swap_events
    WHERE volume_usd IS NOT NULL AND volume_usd > 0
    GROUP BY sender
) sub
WHERE t.address = sub.sender;
