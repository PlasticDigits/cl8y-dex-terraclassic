-- #1391: legacy averages lack proven quorum and are intentionally not promoted.
CREATE TABLE oracle_state (
    ticker VARCHAR(16) PRIMARY KEY CHECK (ticker IN ('ustc', 'lunc', 'vfdusd')),
    price_usd NUMERIC(38,18),
    quality VARCHAR(16) NOT NULL CHECK (quality IN ('fresh', 'degraded', 'unavailable')),
    accepted_sources TEXT[] NOT NULL DEFAULT '{}',
    as_of TIMESTAMPTZ,
    observed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK ((quality = 'fresh' AND price_usd IS NOT NULL AND price_usd >= 0.000000000001 AND price_usd <= 1000000
            AND (cardinality(accepted_sources) = 2
                 OR (cardinality(accepted_sources) = 3 AND accepted_sources @> ARRAY['kucoin','mexc','coingecko']::TEXT[]))
            AND accepted_sources <@ ARRAY['kucoin','mexc','coingecko']::TEXT[]
            AND (ticker <> 'vfdusd' OR NOT ('kucoin' = ANY(accepted_sources)))
            AND (accepted_sources @> ARRAY['kucoin','mexc']::TEXT[]
              OR accepted_sources @> ARRAY['kucoin','coingecko']::TEXT[]
              OR accepted_sources @> ARRAY['mexc','coingecko']::TEXT[])
            AND as_of IS NOT NULL)
        OR (quality <> 'fresh' AND price_usd IS NULL AND cardinality(accepted_sources) = 0 AND as_of IS NULL))
);

-- Only new validated aggregates receive a certificate. Old rows remain empty.
ALTER TABLE oracle_prices ADD COLUMN accepted_sources TEXT[] NOT NULL DEFAULT '{}';

ALTER TABLE oracle_prices ADD CONSTRAINT oracle_prices_certified_sources CHECK (
    cardinality(accepted_sources) = 0 OR (
        source = 'average'
        AND accepted_sources <@ ARRAY['kucoin','mexc','coingecko']::TEXT[]
            AND (ticker <> 'vfdusd' OR NOT ('kucoin' = ANY(accepted_sources)))
        AND (cardinality(accepted_sources) = 2
          OR (cardinality(accepted_sources) = 3 AND accepted_sources @> ARRAY['kucoin','mexc','coingecko']::TEXT[]))
        AND (accepted_sources @> ARRAY['kucoin','mexc']::TEXT[]
          OR accepted_sources @> ARRAY['kucoin','coingecko']::TEXT[]
          OR accepted_sources @> ARRAY['mexc','coingecko']::TEXT[])
        AND price_usd >= 0.000000000001 AND price_usd <= 1000000
    )
);
