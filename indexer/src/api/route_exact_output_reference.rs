//! Bounded integer fair cross-rate for receive sizing (#1376). This mirrors the shared
//! one-whole-token reference policy without starting nested hybrid solvers or loading books.

use std::collections::{HashMap, HashSet};

use axum::http::StatusCode;
use bigdecimal::{ToPrimitive, num_bigint::BigUint};

use super::{
    AppState, best_execution, community_tax_rank as tax, db_orderbook_sim as dbsim,
    route_exact_output::{MAX_GRAPH_PAIRS, PATH_WORK_BUDGET},
    route_graph::RouteGraphSnapshot,
    route_paths, route_slippage,
    route_solver::{self, RouteHop},
};

type Error = (StatusCode, String);

pub(crate) fn supported_hops(snapshot: &RouteGraphSnapshot, hops: &[RouteHop]) -> bool {
    hops.iter().all(|h| {
        [&h.offer_token, &h.ask_token].iter().all(|token| {
            snapshot.assets.iter().any(|a| {
                a.contract_address.as_ref() == Some(*token)
                    && a.is_cw20
                    && (0..=18).contains(&a.decimals)
                    && !crate::indexer::defillama::is_columbus5_gem_address(token)
            })
        })
    })
}

fn decimals(snapshot: &RouteGraphSnapshot, token: &str) -> Option<u32> {
    snapshot
        .assets
        .iter()
        .find(|a| {
            a.contract_address
                .as_deref()
                .is_some_and(|a| a.eq_ignore_ascii_case(token))
        })
        .and_then(|a| (0..=18).contains(&a.decimals).then_some(a.decimals as u32))
}

pub(crate) struct FairReference {
    price_in: u128,
    price_out: u128,
    decimals_in: u32,
    decimals_out: u32,
    decimals_quote: u32,
    used_pairs: Vec<String>,
}

pub(crate) struct FairFields {
    pub spot: String,
    pub slippage: String,
    pub price_in: String,
    pub price_out: String,
    pub extreme: bool,
}

impl FairReference {
    pub(crate) fn is_fresh(
        &self,
        mirrors: &HashMap<String, dbsim::HopMirror>,
        max_age: u64,
    ) -> bool {
        self.used_pairs.iter().all(|p| {
            mirrors.get(p).is_some_and(|m| {
                m.freshness == dbsim::MirrorFreshness::Fresh
                    && !dbsim::is_snapshot_stale(m.snapshot_at, max_age)
            })
        })
    }

    pub(crate) fn fields(&self, offer: u128, actual: u128) -> Option<FairFields> {
        let numerator = BigUint::from(offer)
            * BigUint::from(self.price_in)
            * BigUint::from(10u128.checked_pow(self.decimals_out)?);
        let denominator =
            BigUint::from(10u128.checked_pow(self.decimals_in)?) * BigUint::from(self.price_out);
        if denominator == BigUint::from(0u32) {
            return None;
        }
        // Same nearest-raw-unit advisory rounding as the shared reference, using wide integers.
        let spot = ((numerator + &denominator / BigUint::from(2u32)) / denominator).to_u128()?;
        if spot == 0 || actual == 0 {
            return None;
        }
        let (lo, hi) = (actual.min(spot), actual.max(spot));
        let percent_hundredths = ((BigUint::from(hi - lo) * BigUint::from(10000u32)
            + BigUint::from(hi / 2))
            / BigUint::from(hi))
        .to_u128()?;
        Some(FairFields {
            spot: spot.to_string(),
            slippage: format!(
                "{}.{:02}",
                percent_hundredths / 100,
                percent_hundredths % 100
            ),
            price_in: price_string(self.price_in, self.decimals_quote)?,
            price_out: price_string(self.price_out, self.decimals_quote)?,
            extreme: BigUint::from(lo) * BigUint::from(100u32) <= BigUint::from(hi),
        })
    }
}

fn price_string(raw: u128, decimal_count: u32) -> Option<String> {
    let scale = 10u128.checked_pow(decimal_count)?;
    let fraction = (BigUint::from(raw % scale) * BigUint::from(10u128.pow(18))
        / BigUint::from(scale))
    .to_u128()?;
    Some(format!("{}.{fraction:018}", raw / scale))
}

async fn reference_paths(
    snapshot: &RouteGraphSnapshot,
    token: &str,
    quote: &str,
) -> Result<Vec<Vec<RouteHop>>, Error> {
    if token.eq_ignore_ascii_case(quote) {
        return Ok(vec![]);
    }
    let start = route_solver::resolve_id(&snapshot.addr_to_id, token).ok_or_else(unavailable)?;
    let goal = route_solver::resolve_id(&snapshot.addr_to_id, quote).ok_or_else(unavailable)?;
    let pairs = snapshot.pairs.clone();
    let (paths, _) = tokio::task::spawn_blocking(move || {
        route_paths::find_paths_top_k_with_work_budget(
            start,
            goal,
            &pairs,
            route_solver::GET_DEFAULT_MAX_HOPS,
            best_execution::PATH_ENUM_POOL,
            PATH_WORK_BUDGET,
        )
    })
    .await
    .map_err(super::internal_err)?;
    paths
        .iter()
        .map(|p| route_solver::build_hops_and_ops(p, &snapshot.id_to_addr).map(|(h, _)| h))
        .collect::<Result<Vec<_>, _>>()
        .map(|paths| {
            paths
                .into_iter()
                .filter(|h| supported_hops(snapshot, h))
                .collect()
        })
}

fn unavailable() -> Error {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "Fresh fair-route reference unavailable; retry after reserves refresh".into(),
    )
}

fn price_from_paths(
    paths: &[Vec<RouteHop>],
    mirrors: &HashMap<String, dbsim::HopMirror>,
    one_unit: u128,
    discount: u16,
    taxes: &tax::TaxRankSnapshot,
    token: &str,
) -> Option<(u128, Vec<String>)> {
    let mut used = 0;
    let mut best = None;
    for hops in paths {
        if tax::path_sells_middle_tax_hop(hops, token, taxes) {
            continue;
        }
        let output = hops.iter().try_fold(one_unit, |offer, h| {
            let m = mirrors.get(&h.pair)?;
            let funded = taxes.path_route_local(hops);
            let economic = if funded {
                tax::sell_economic(offer, taxes.sell_bps(&h.offer_token))
            } else {
                offer
            };
            let out = dbsim::simulate_pool_only_from_mirror(m, &h.offer_token, economic, discount)
                .ok()?;
            let out = if funded {
                tax::buy_split_net(out, taxes.buy_bps(&h.ask_token))
            } else {
                out
            };
            let reserve = if h.offer_token.eq_ignore_ascii_case(&m.asset_0_addr) {
                m.reserve_1
            } else {
                m.reserve_0
            };
            if out == 0 || dbsim::hop_near_full_ask_drain(out, reserve) {
                None
            } else {
                Some(out)
            }
        });
        let Some(output) = output else {
            continue;
        };
        used += 1;
        if best.as_ref().is_none_or(|(out, _)| output > *out) {
            best = Some((output, hops.iter().map(|h| h.pair.clone()).collect()));
        }
        if used == best_execution::MAX_PATH_CANDIDATES {
            break;
        }
    }
    best
}

pub(crate) async fn load(
    state: &AppState,
    snapshot: &RouteGraphSnapshot,
    token_in: &str,
    token_out: &str,
    discount: u16,
    taxes: &tax::TaxRankSnapshot,
    mirrors: &mut HashMap<String, dbsim::HopMirror>,
) -> Result<FairReference, Error> {
    if snapshot.pairs.len() > MAX_GRAPH_PAIRS {
        return Err(unavailable());
    }
    // Networks without the standard wrapped quote use token_in as the local reference asset.
    // A present standard quote is never silently replaced when its paths are unavailable.
    let quote = if let Some((quote, _)) = route_slippage::find_quote_token(&snapshot.assets) {
        let selected = snapshot
            .assets
            .iter()
            .find(|a| a.contract_address.as_deref() == Some(&quote))
            .ok_or_else(unavailable)?;
        let matches: HashSet<_> = snapshot
            .assets
            .iter()
            .filter(|a| {
                a.is_cw20
                    && a.symbol.eq_ignore_ascii_case(&selected.symbol)
                    && !a
                        .contract_address
                        .as_deref()
                        .is_some_and(crate::indexer::defillama::is_columbus5_gem_address)
            })
            .filter_map(|a| a.contract_address.as_deref())
            .collect();
        if matches.len() != 1 {
            return Err(unavailable());
        }
        quote
    } else {
        token_in.to_string()
    };
    let decimals_in = decimals(snapshot, token_in).ok_or_else(unavailable)?;
    let decimals_out = decimals(snapshot, token_out).ok_or_else(unavailable)?;
    let decimals_quote = decimals(snapshot, &quote).ok_or_else(unavailable)?;
    if crate::indexer::defillama::is_columbus5_gem_address(&quote) {
        return Err(unavailable());
    }
    let in_paths = reference_paths(snapshot, token_in, &quote).await?;
    let out_paths = reference_paths(snapshot, token_out, &quote).await?;
    let extras: Vec<_> = in_paths
        .iter()
        .chain(out_paths.iter())
        .flat_map(|h| tax::extra_addrs_from_hops(h))
        .collect();
    // Reference paths use the same live trading capabilities as executable paths. Never
    // overwrite upgraded instance metadata with a narrower/stale Create catalog row.
    let mut reference_taxes =
        tax::try_load_tax_rank_snapshot(state, token_in, token_out, &extras, None)
            .await
            .map_err(super::internal_err)?;
    reference_taxes
        .exempt_tokens
        .extend(taxes.exempt_tokens.iter().cloned());
    let addrs: Vec<_> = in_paths
        .iter()
        .chain(out_paths.iter())
        .flatten()
        .map(|h| h.pair.clone())
        .filter(|p| !mirrors.contains_key(p))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    // <= 2 × 32 × 4 distinct pairs. Pool-only snapshots never fetch books.
    mirrors.extend(
        dbsim::preload_pool_mirrors_for_pairs(
            &state.pool,
            &addrs,
            &snapshot.id_to_addr,
            state.book_snapshot_max_staleness_ms,
        )
        .await
        .map_err(super::internal_err)?,
    );
    let one_quote = 10u128.checked_pow(decimals_quote).ok_or_else(unavailable)?;
    let (price_in, mut used_pairs) = if token_in.eq_ignore_ascii_case(&quote) {
        (one_quote, vec![])
    } else {
        price_from_paths(
            &in_paths,
            mirrors,
            10u128.checked_pow(decimals_in).ok_or_else(unavailable)?,
            discount,
            &reference_taxes,
            token_in,
        )
        .ok_or_else(unavailable)?
    };
    let (price_out, out_pairs) = if token_out.eq_ignore_ascii_case(&quote) {
        (one_quote, vec![])
    } else {
        price_from_paths(
            &out_paths,
            mirrors,
            10u128.checked_pow(decimals_out).ok_or_else(unavailable)?,
            discount,
            &reference_taxes,
            token_out,
        )
        .ok_or_else(unavailable)?
    };
    used_pairs.extend(out_pairs);
    let reference = FairReference {
        price_in,
        price_out,
        decimals_in,
        decimals_out,
        decimals_quote,
        used_pairs,
    };
    if !reference.is_fresh(mirrors, state.book_snapshot_max_staleness_ms) {
        return Err(unavailable());
    }
    Ok(reference)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_exact_output_fair_crossrate_is_integer_symmetric_and_decimal_aware() {
        let reference = FairReference {
            price_in: 2_000_000,
            price_out: 4_000_000,
            decimals_in: 6,
            decimals_out: 18,
            decimals_quote: 6,
            used_pairs: vec![],
        };
        let fair = reference
            .fields(2_000_000, 1_000_000_000_000_000_000)
            .unwrap();
        assert_eq!(fair.spot, "1000000000000000000");
        assert_eq!(fair.slippage, "0.00");
        assert_eq!(fair.price_in, "2.000000000000000000");
        assert_eq!(
            reference
                .fields(2_000_000, 500_000_000_000_000_000)
                .unwrap()
                .slippage,
            "50.00"
        );
        assert_eq!(
            reference
                .fields(2_000_000, 2_000_000_000_000_000_000)
                .unwrap()
                .slippage,
            "50.00"
        );
        assert!(
            reference
                .fields(2_000_000, 100_000_000_000_000_000_000)
                .unwrap()
                .extreme
        );
        assert!(
            !reference
                .fields(2_000_000, 99_000_000_000_000_000_000)
                .unwrap()
                .extreme
        );
    }

    #[test]
    fn route_exact_output_fair_crossrate_rejects_zero_and_checked_narrowing_overflow() {
        let mut reference = FairReference {
            price_in: u128::MAX,
            price_out: 1,
            decimals_in: 0,
            decimals_out: 18,
            decimals_quote: 6,
            used_pairs: vec![],
        };
        assert!(reference.fields(u128::MAX, 1).is_none());
        reference.price_in = 1;
        reference.decimals_out = 0;
        assert!(reference.fields(0, 1).is_none());
        // A tiny offer can round to zero even when a larger alternate has a valid reference.
        reference.price_out = 1_000_000;
        assert!(reference.fields(1, 1).is_none());
        assert!(reference.fields(1_000_000, 1).is_some());
        reference.price_out = 0;
        assert!(reference.fields(1, 1).is_none());
    }
}
