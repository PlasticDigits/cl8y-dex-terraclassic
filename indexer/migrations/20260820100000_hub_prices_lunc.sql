-- GitLab #570: LUNC hub ticker (CEX wrap 1:1) on materialized hub_prices.
-- Display-only on /protocol DEX hub card. Not a fourth CEX oracle tab.

ALTER TABLE hub_prices DROP CONSTRAINT hub_prices_ticker_chk;
ALTER TABLE hub_prices ADD CONSTRAINT hub_prices_ticker_chk
    CHECK (ticker IN ('custc', 'lunc', 'ust1', 'ustr'));

COMMENT ON TABLE hub_prices IS
    'DEX hub USD marks (#556 / #570). usd(cUSTC)=USTC oracle; usd(LUNC)=LUNC oracle wrap 1:1; UST1/USTR from max USD-TVL factory reserves. Advisory, not settlement.';
