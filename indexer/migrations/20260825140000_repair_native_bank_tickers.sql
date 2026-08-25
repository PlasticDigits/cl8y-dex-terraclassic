-- GitLab #630: retail tickers for bank uluna / uusd.
-- Only rewrite native rows whose catalog still stores the denom as symbol.
-- Wrap CW20s (cLUNC / cUSTC) are contract_address rows and stay untouched.

UPDATE assets
SET name = 'Terra Luna Classic',
    symbol = 'LUNC',
    updated_at = NOW()
WHERE denom = 'uluna'
  AND is_cw20 = false
  AND contract_address IS NULL
  AND lower(symbol) = 'uluna';

UPDATE assets
SET name = 'TerraClassicUSD',
    symbol = 'USTC',
    updated_at = NOW()
WHERE denom = 'uusd'
  AND is_cw20 = false
  AND contract_address IS NULL
  AND lower(symbol) = 'uusd';
