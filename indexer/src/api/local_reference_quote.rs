//! Address-bound independent fair prices frozen with liquidity generations (#1452).
//! Missing/stale marks fail explicitly. Selected-route spot never substitutes for a
//! fair reference, and no request recursively quotes one whole token over the network.
use crate::{
    db::queries::{assets, hub_prices},
    indexer::hub_usd::HubUsdConfig,
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct ReferencePrice {
    pub price: BigDecimal,
    pub decimals: u32,
    pub as_of: DateTime<Utc>,
}
#[derive(Clone, Debug, Default)]
pub struct ReferencePrices {
    pub prices: HashMap<i32, ReferencePrice>,
}
impl ReferencePrices {
    pub fn fair_output(
        &self,
        input_id: i32,
        output_id: i32,
        input: u128,
        now: DateTime<Utc>,
    ) -> Option<(u128, i64)> {
        let a = self.prices.get(&input_id)?;
        let b = self.prices.get(&output_id)?;
        if [a, b].iter().any(|p| {
            p.decimals > 38
                || p.price <= BigDecimal::from(0)
                || p.as_of > now
                || now.signed_duration_since(p.as_of).num_milliseconds() > 20_000
        }) {
            return None;
        }
        let output = (BigDecimal::from(input)
            * &a.price
            * BigDecimal::from(10u128.checked_pow(b.decimals)?)
            / (&b.price * BigDecimal::from(10u128.checked_pow(a.decimals)?)))
        .with_scale(0)
        .to_string()
        .parse::<u128>()
        .ok()?;
        (output > 0).then_some((output, a.as_of.min(b.as_of).timestamp_millis()))
    }
    pub async fn observe(
        pool: &PgPool,
        assets: &[assets::AssetRow],
        hub: &HubUsdConfig,
    ) -> Result<Self, sqlx::Error> {
        let mut prices = HashMap::new();
        let indexed = assets.iter().map(|a| (a.id, a)).collect::<HashMap<_, _>>();
        for row in hub_prices::get_all_hub_prices(pool).await? {
            if !hub_prices::row_usable(&row, Utc::now()) {
                continue;
            }
            let Some(id) = row.asset_id else {
                continue;
            };
            let Some(asset) = indexed.get(&id) else {
                continue;
            };
            let expected = match row.ticker.as_str() {
                "custc" => &hub.custc_address,
                "lunc" => &hub.clunc_address,
                "ust1" => &hub.ust1_address,
                "ustr" => &hub.ustr_address,
                _ => continue,
            };
            if asset.contract_address.as_deref() != Some(expected.as_str()) {
                continue;
            }
            let Some(as_of) = row.as_of else {
                continue;
            };
            let Ok(decimals) = u32::try_from(asset.decimals) else {
                continue;
            };
            if decimals > 38 {
                continue;
            }
            prices.insert(
                id,
                ReferencePrice {
                    price: row.price_usd,
                    decimals,
                    as_of,
                },
            );
        }
        let marks:Vec<(String,BigDecimal,DateTime<Utc>)>=sqlx::query_as("SELECT contract_address,price_usd,as_of FROM economic_token_marks WHERE price_usd>0 AND as_of BETWEEN NOW()-INTERVAL '20 seconds' AND NOW() AND EXISTS(SELECT 1 FROM oracle_state WHERE quality='fresh' AND ticker=CASE WHEN economic_token_marks.anchor_ticker='lunc' THEN 'lunc' ELSE 'ustc' END AND as_of BETWEEN NOW()-INTERVAL '300 seconds' AND NOW()) AND (anchor_ticker IN ('custc','lunc') OR EXISTS(SELECT 1 FROM hub_price_references WHERE ticker=economic_token_marks.anchor_ticker AND quality='bounded_spot')) LIMIT 2048").fetch_all(pool).await?;
        for (address, price, as_of) in marks {
            let Some(asset) = assets
                .iter()
                .find(|a| a.contract_address.as_deref() == Some(address.as_str()))
            else {
                continue;
            };
            let Ok(decimals) = u32::try_from(asset.decimals) else {
                continue;
            };
            if decimals <= 38 {
                prices.insert(
                    asset.id,
                    ReferencePrice {
                        price,
                        decimals,
                        as_of,
                    },
                );
            }
        }
        Ok(Self { prices })
    }
    #[doc(hidden)]
    pub fn synthetic(ids: impl Iterator<Item = i32>) -> Self {
        Self {
            prices: ids
                .map(|id| {
                    (
                        id,
                        ReferencePrice {
                            price: BigDecimal::from(1),
                            decimals: 0,
                            as_of: Utc::now(),
                        },
                    )
                })
                .collect(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fair_prices_are_independent_checked_and_decimal_aware() {
        let now = Utc::now();
        let refs = ReferencePrices {
            prices: HashMap::from([
                (
                    1,
                    ReferencePrice {
                        price: BigDecimal::from(2),
                        decimals: 18,
                        as_of: now,
                    },
                ),
                (
                    2,
                    ReferencePrice {
                        price: BigDecimal::from(1),
                        decimals: 6,
                        as_of: now,
                    },
                ),
            ]),
        };
        assert_eq!(
            refs.fair_output(1, 2, 10u128.pow(18), now).unwrap().0,
            2_000_000
        );
        assert_eq!(
            refs.fair_output(2, 1, 1_000_000, now).unwrap().0,
            500_000_000_000_000_000
        );
        assert!(refs.fair_output(1, 2, 1, now).is_none());
        assert!(refs
            .fair_output(1, 2, 10u128.pow(18), now + chrono::Duration::seconds(21))
            .is_none());
    }
}
