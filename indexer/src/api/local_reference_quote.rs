//! Address-bound independent fair prices frozen with liquidity generations (#1452).
//! Missing/stale marks fail explicitly. Selected-route spot never substitutes for a
//! fair reference, and no request recursively quotes one whole token over the network.
use crate::{
    db::queries::{assets, hub_prices},
    indexer::{
        economic_usd::economic_floor_exclusions,
        hub_usd::{HubMark, HubTicker, HubUsdConfig, HubUsdSnapshot},
    },
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
    /// Original-time scheduled policy evidence; never an executable price.
    pub policy_exclusions: HashMap<i32, DateTime<Utc>>,
    pub policy_identity: String,
}
impl ReferencePrices {
    pub fn policy_identity(hub: &HubUsdConfig) -> String {
        format!(
            "economic-total-tvl-v2|hub_depth={}|economic_total={}",
            hub.tvl_floor.normalized(),
            hub.economic_tvl_floor.normalized()
        )
    }
    /// Fixed public reason classes only; every read retains original freshness.
    pub fn unavailable_reason(&self, id: i32, now: DateTime<Utc>) -> Option<&'static str> {
        let Some(price) = self.prices.get(&id) else {
            return Some(if self.policy_excluded(id, now) {
                "reference_policy_excluded"
            } else {
                "missing_observations"
            });
        };
        if price.decimals > 38 || price.price <= BigDecimal::from(0) {
            Some("unsupported_semantics")
        } else if price.as_of > now
            || now.signed_duration_since(price.as_of).num_milliseconds() > 20_000
        {
            Some("stale_observations")
        } else {
            None
        }
    }
    pub fn policy_excluded(&self, id: i32, now: DateTime<Utc>) -> bool {
        !self.prices.contains_key(&id)
            && self.policy_exclusions.get(&id).is_some_and(|at| {
                *at <= now && now.signed_duration_since(*at).num_milliseconds() <= 20_000
            })
    }
    pub fn fair_output(
        &self,
        input_id: i32,
        output_id: i32,
        input: u128,
        now: DateTime<Utc>,
    ) -> Option<(u128, i64)> {
        let a = self.prices.get(&input_id)?;
        let b = self.prices.get(&output_id)?;
        if [input_id, output_id]
            .into_iter()
            .any(|id| self.unavailable_reason(id, now).is_some())
        {
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
        // Scheduled publication replaces hub and economic marks atomically.
        // One read-only snapshot prevents old anchor USD/new token USD splices
        // during a concurrent rescaling; original observation expiry still wins.
        let mut tx = pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await?;
        let mut prices = HashMap::new();
        let mut qualified_hubs = HubUsdSnapshot::default();
        let indexed = assets.iter().map(|a| (a.id, a)).collect::<HashMap<_, _>>();
        for row in hub_prices::get_all_hub_prices(&mut *tx).await? {
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
            // The diagnostic producer uses exactly the originally admitted
            // anchors, with no reserve-selected substitute or timestamp renewal.
            let mark = HubMark {
                ticker: HubTicker::parse(&row.ticker).expect("allowlisted above"),
                asset_id: Some(id),
                price_usd: row.price_usd.clone(),
                source_pair_id: row.source_pair_id,
                source_pair_address: row.source_pair_address.clone(),
                tvl_usd: row.tvl_usd.clone(),
                priced_leg_depth_usd: row.priced_leg_depth_usd.clone(),
                as_of: Some(as_of),
            };
            match row.ticker.as_str() {
                "custc" => qualified_hubs.custc = Some(mark),
                "lunc" => qualified_hubs.lunc = Some(mark),
                "ust1" => qualified_hubs.ust1 = Some(mark),
                "ustr" => qualified_hubs.ustr = Some(mark),
                _ => unreachable!(),
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
        let marks: Vec<(String, BigDecimal, DateTime<Utc>, String)> = sqlx::query_as(
            "SELECT m.contract_address,m.price_usd,m.as_of,m.anchor_ticker
             FROM economic_token_marks m
             JOIN assets token ON token.id=m.asset_id AND token.is_cw20
               AND token.denom IS NULL AND token.contract_address=m.contract_address
             JOIN pairs p ON p.id=m.source_pair_id AND p.contract_address=m.source_pair_address
             JOIN assets a ON a.id=CASE WHEN p.asset_0_id=m.asset_id THEN p.asset_1_id
                 WHEN p.asset_1_id=m.asset_id THEN p.asset_0_id END
             WHERE m.price_usd>0 AND m.tvl_usd >= $1
               AND m.as_of BETWEEN NOW()-INTERVAL '20 seconds' AND NOW()
               AND ((a.is_cw20 AND a.denom IS NULL
                 AND a.contract_address=CASE m.anchor_ticker WHEN 'custc' THEN $2
                   WHEN 'lunc' THEN $3 WHEN 'ust1' THEN $4 WHEN 'ustr' THEN $5 END)
                 OR (NOT a.is_cw20 AND a.contract_address IS NULL
                   AND a.denom=CASE m.anchor_ticker WHEN 'custc' THEN 'uusd' WHEN 'lunc' THEN 'uluna' END))
               AND EXISTS(SELECT 1 FROM oracle_state os WHERE os.quality='fresh'
                 AND cardinality(os.accepted_sources)>=2
                 AND os.ticker=CASE WHEN m.anchor_ticker='lunc' THEN 'lunc' ELSE 'ustc' END
                 AND os.as_of BETWEEN NOW()-INTERVAL '300 seconds' AND NOW())
               AND (m.anchor_ticker IN ('custc','lunc') OR EXISTS(
                 SELECT 1 FROM hub_prices hp WHERE hp.ticker=m.anchor_ticker
                   AND hp.price_usd > 0
                   AND hp.as_of BETWEEN NOW() - INTERVAL '20 seconds' AND NOW()))
             ORDER BY m.asset_id LIMIT 2048"
        )
        .bind(&hub.economic_tvl_floor)
        .bind(&hub.custc_address).bind(&hub.clunc_address)
        .bind(&hub.ust1_address).bind(&hub.ustr_address)
        .fetch_all(&mut *tx).await?;
        for (address, price, as_of, anchor_ticker) in marks {
            let qualified_anchor = match anchor_ticker.as_str() {
                "custc" => &qualified_hubs.custc,
                "lunc" => &qualified_hubs.lunc,
                "ust1" => &qualified_hubs.ust1,
                "ustr" => &qualified_hubs.ustr,
                _ => continue,
            };
            let Some(anchor_at) = qualified_anchor.as_ref().and_then(|mark| mark.as_of) else {
                continue;
            };
            let now = Utc::now();
            if anchor_at > now || now.signed_duration_since(anchor_at).num_milliseconds() > 20_000 {
                continue;
            }
            let as_of = as_of.min(anchor_at);
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
        let now = Utc::now();
        let policy_exclusions = hub_prices::quote_reference_census(&mut *tx)
            .await?
            .map(|pairs| economic_floor_exclusions(now, hub, &pairs, &qualified_hubs))
            .unwrap_or_default()
            .into_iter()
            .filter(|(id, _)| !prices.contains_key(id))
            .collect::<HashMap<_, _>>();
        tx.commit().await?;
        tracing::debug!(
            reference_policy_excluded = policy_exclusions.len(),
            qualified_references = prices.len(),
            policy = "economic-total-tvl-v2",
            economic_total_tvl_floor = %hub.economic_tvl_floor,
            hub_priced_leg_floor = %hub.tvl_floor,
            "Scheduled execution reference diagnosis (#1479)"
        );
        Ok(Self {
            prices,
            policy_exclusions,
            policy_identity: Self::policy_identity(hub),
        })
    }
    #[doc(hidden)]
    pub fn synthetic(ids: impl Iterator<Item = i32>) -> Self {
        Self {
            policy_exclusions: HashMap::new(),
            policy_identity: "synthetic".into(),
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
    fn issue1479_policy_evidence_never_prices_and_expires_without_renewal() {
        let now = Utc::now();
        let mut refs = ReferencePrices::default();
        refs.policy_exclusions.insert(1, now);
        assert!(refs.policy_excluded(1, now));
        assert!(refs.fair_output(1, 2, 1_000_000, now).is_none());
        assert!(!refs.policy_excluded(1, now - chrono::Duration::milliseconds(1)));
        assert!(!refs.policy_excluded(1, now + chrono::Duration::milliseconds(20_001)));
        // New admitted provenance supersedes exclusion; no permanent latch.
        refs.prices.insert(
            1,
            ReferencePrice {
                price: 2.into(),
                decimals: 6,
                as_of: now,
            },
        );
        assert!(!refs.policy_excluded(1, now));
    }

    #[test]
    fn issue1479_policy_identity_binds_exact_independent_thresholds() {
        let cfg = HubUsdConfig {
            custc_address: "terra1custc".into(),
            clunc_address: "terra1clunc".into(),
            ust1_address: "terra1ust1".into(),
            ustr_address: "terra1ustr".into(),
            cl8y_address: "terra1cl8y".into(),
            usdt_address: "terra1usdt".into(),
            tvl_floor: 100.into(),
            economic_tvl_floor: 10.into(),
            max_staleness: std::time::Duration::from_secs(20),
        };
        let first = ReferencePrices::policy_identity(&cfg);
        let mut changed = cfg.clone();
        changed.economic_tvl_floor = 100.into();
        assert_ne!(first, ReferencePrices::policy_identity(&changed));
        changed = cfg.clone();
        changed.tvl_floor = 10.into();
        assert_ne!(first, ReferencePrices::policy_identity(&changed));
        changed = cfg;
        changed.economic_tvl_floor = "10.000".parse().unwrap();
        assert_eq!(first, ReferencePrices::policy_identity(&changed));
    }
    #[test]
    fn fair_prices_are_independent_checked_and_decimal_aware() {
        let now = Utc::now();
        let refs = ReferencePrices {
            policy_exclusions: HashMap::new(),
            policy_identity: "synthetic".into(),
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
