-- GitLab #1258: NULL-only swap USD for the columbus-5 registry USDT CW20.
--
-- Advisory $1 on the pinned contract only (not symbol=USDT, not Peg1 / hub UST1).
-- Never UPDATE non-NULL price_usd / volume_usd (GitLab #568). UST1/cUSTC history
-- is untouched. LocalTerra uses indexer `USDT_CW20_ADDRESS` rust backfill, not this pin.
-- Candle rebuild is the indexer startup hook (same pin / env overlay).

UPDATE swap_events se
SET price_usd = se.price
FROM pairs p
JOIN assets q ON q.id = p.asset_1_id
WHERE se.pair_id = p.id
  AND se.price_usd IS NULL
  AND se.price > 0
  AND se.price < POWER(10::numeric, 20)
  AND q.is_cw20
  AND lower(q.contract_address) = 'terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4';

UPDATE swap_events se
SET volume_usd = x.usd
FROM (
    SELECT se2.id,
           CASE
               WHEN se2.offer_asset_id = u.id
                    AND u.decimals BETWEEN 0 AND 38
                   THEN se2.offer_amount / POWER(10::numeric, u.decimals)
               WHEN se2.ask_asset_id = u.id
                    AND u.decimals BETWEEN 0 AND 38
                   THEN se2.return_amount / POWER(10::numeric, u.decimals)
               ELSE NULL
           END AS usd
    FROM swap_events se2
    JOIN assets u ON u.is_cw20
      AND lower(u.contract_address) = 'terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4'
    WHERE se2.volume_usd IS NULL
) x
WHERE se.id = x.id
  AND x.usd IS NOT NULL
  AND x.usd > 0
  AND x.usd < POWER(10::numeric, 20)
  AND se.volume_usd IS NULL;
