//! Advisory selected-route pricing (#1500). Only the selected original mirrors
//! are walked again, with all charges removed; this cannot select or admit a route.
use super::{sim, wide_floor, BigInt, Candidate, Value};
use serde_json::json;

const METHOD: &str = "selected_route_fee_free_v1";

pub(super) fn unavailable() -> Value {
    json!({"version":1,"status":"unavailable","method":METHOD})
}

/// Attach the selected original bounded book view once, after selection. Doing
/// this in the scorer would allocate up to 500 rows for every search candidate.
pub(super) fn attach_original_book_terms(candidate: &Candidate, body: &mut Value) {
    let Some(hops) = body["execution_metadata"]["hops"].as_array_mut() else {
        return;
    };
    for (index, hop) in hops.iter_mut().enumerate() {
        let Some(mirror) = candidate.mirrors.get(index) else {
            continue;
        };
        let Some(offer) = hop["offer_token"].as_str() else {
            continue;
        };
        let budget = candidate.operations[index]["terra_swap"]["hybrid"]["book_input"]
            .as_str()
            .and_then(|value| value.parse::<u128>().ok())
            .unwrap_or(0);
        let terms: Vec<_> = if budget == 0 {
            vec![]
        } else {
            sim::fee_free_book_terms(mirror, offer).into_iter()
                .map(|(remaining,price)|json!({"remaining":remaining.to_string(),"price_atomics":price.to_string()})).collect()
        };
        hop["book_terms"] = json!(terms);
    }
}

fn projected(mut amount: u128, stages: &[Value]) -> Option<u128> {
    for stage in stages {
        amount = wide_floor(
            amount,
            stage["reserve_out"].as_str()?.parse().ok()?,
            stage["reserve_in"].as_str()?.parse().ok()?,
        )?;
    }
    Some(amount)
}

fn diagnostic(
    body: &Value,
    stages: Vec<Value>,
    baseline: u128,
    fee_free: u128,
    fees: u128,
) -> Option<Value> {
    let m = &body["execution_metadata"];
    let a = &body["economic_attribution"];
    if baseline == 0 || fee_free == 0 || a["status"] != "available" {
        return None;
    }
    let cost: BigInt = a["components"]["trading_cost"].as_str()?.parse().ok()?;
    let tax_hook = cost - BigInt::from(fees);
    if tax_hook < BigInt::from(0) {
        return None;
    }
    Some(json!({
        "version":1,"status":"available","method":METHOD,
        "generation":m["generation"],"observed_at_ms":m["observed_at_ms"],"expires_at_ms":m["expires_at_ms"],
        "input_amount":m["input_amount"],"policy":m["policy"],"max_spread":m["max_spread"],
        "router_operations":body["router_operations"],"stages":stages,
        "baseline_amount_out":baseline.to_string(),"fee_free_amount_out":fee_free.to_string(),
        "order_movement":(BigInt::from(baseline)-BigInt::from(fee_free)).to_string(),
        "trading_fee":fees.to_string(),"tax_hook_cost":tax_hook.to_string(),
        "wrapper_cost":a["components"]["wrapper_loss"],"reference_divergence":a["components"]["reference_divergence"]
    }))
}

pub(super) fn route(candidate: &Candidate, gross: u128, body: &Value) -> Value {
    let result = (|| {
        if candidate.path.len() != candidate.mirrors.len() || candidate.path.len() > 4 {
            return None;
        }
        let mut output = gross;
        let mut stages = Vec::with_capacity(candidate.path.len());
        for (index, mirror) in candidate.mirrors.iter().enumerate() {
            let hop = &candidate.hops[index];
            let offer = hop["offer_token"].as_str()?;
            let economic: u128 = hop["economic_offer_amount"].as_str()?.parse().ok()?;
            let hybrid = &candidate.operations[index]["terra_swap"]["hybrid"];
            let (book, fills) = if hybrid.is_null() {
                (0, 1)
            } else {
                let budget: u128 = hybrid["book_input"].as_str()?.parse().ok()?;
                (
                    wide_floor(output, budget, economic)?,
                    hybrid["max_maker_fills"].as_u64()?.try_into().ok()?,
                )
            };
            // The mapper admitted here exchanges equal raw principal units.
            // Removing its fees/tax restores gross principal, not credited input.
            let mut fee_free = (**mirror).clone();
            fee_free.fee_bps = 0;
            let leg = sim::quote_local_leg(&fee_free, offer, output, book, fills).ok()?;
            let e = &candidate.economics[index];
            stages.push(json!({"reserve_in":e.reserve_in.to_string(),"reserve_out":e.reserve_out.to_string(),
                "input_amount":output.to_string(),"output_amount":leg.return_amount.to_string(),
                "book_input":book.to_string(),"max_maker_fills":fills,
                "book_side":if offer == mirror.asset_0_addr {"bid"} else {"ask"},
                "book":body["execution_metadata"]["hops"][index]["book_terms"]}));
            output = leg.return_amount;
        }
        let baseline = projected(gross, &stages)?;
        let mut fees = 0u128;
        for (index, e) in candidate.economics.iter().enumerate() {
            // Difference of boundaries preserves each suffix's integer floors.
            let before = projected(e.pre_commission?, &stages[index + 1..])?;
            let after = projected(e.post_commission, &stages[index + 1..])?;
            fees = fees.checked_add(before.checked_sub(after)?)?;
        }
        diagnostic(body, stages, baseline, output, fees)
    })();
    result.unwrap_or_else(unavailable)
}

pub(super) fn wrapper_only(gross: u128, body: &Value) -> Value {
    diagnostic(body, vec![], gross, gross, 0).unwrap_or_else(unavailable)
}

#[cfg(test)]
mod tests {
    use super::super::{response, score, score_with_book, tests as fixtures, SCALE};
    use super::*;
    use std::sync::Arc;

    fn parse(v: &Value) -> u128 {
        v.as_str().unwrap().parse().unwrap()
    }
    #[test]
    fn issue1500_one_to_four_hops_use_chained_fee_free_amounts_and_integer_rounding() {
        for count in 1..=4 {
            let mut s = fixtures::fixture();
            s.quote_relative_protection = true;
            let depth = 10u128.pow(18);
            let input = 10_000;
            let mut path = Vec::new();
            let mut expected = input;
            let mut previous = 1;
            for index in 0..count {
                let ask = if index + 1 == count { 2 } else { 100 + index };
                s.addresses.insert(
                    ask,
                    if ask == 2 {
                        "out18".into()
                    } else {
                        format!("raw-stage-{index}")
                    },
                );
                let pair = format!("order-metric-stage-{index}");
                let mut m = (*s.mirrors["pair7a"]).clone();
                m.asset_0_addr = s.addresses[&previous].clone();
                m.asset_1_addr = s.addresses[&ask].clone();
                m.reserve_0 = depth;
                m.reserve_1 = depth;
                m.fee_bps = 0;
                s.mirrors.insert(pair.clone(), Arc::new(m));
                path.push((pair, previous, ask));
                previous = ask;
                expected = expected * depth / (depth + expected);
            }
            let c = score(&s, &path, input, SCALE / 20).unwrap();
            let body = response(&s, &fixtures::request(input), &c, input, SCALE / 20, 1);
            assert_eq!(
                parse(&body["order_economics"]["baseline_amount_out"]),
                input
            );
            assert_eq!(
                parse(&body["order_economics"]["fee_free_amount_out"]),
                expected
            );
            assert_eq!(
                parse(&body["order_economics"]["order_movement"]),
                count as u128
            );
        }
    }

    #[test]
    fn issue1500_pure_wrapper_loss_and_reference_divergence_never_inflate_order_movement() {
        for (gross, net, fair) in [(100, 98, 100), (100, 100, 140), (100, 98, 140)] {
            let mut body = json!({"execution_metadata":{"input_amount":"100","generation":"owned-original","observed_at_ms":10,"expires_at_ms":20,"policy":"wrapper_only","max_spread":"0.05"},"router_operations":[]});
            body["economic_attribution"] =
                super::super::attribution::wrapper_only(gross, fair, net, &body);
            let d = wrapper_only(gross, &body);
            assert_eq!(d["status"], "available");
            assert_eq!(d["baseline_amount_out"], "100");
            assert_eq!(d["order_movement"], "0");
            assert_eq!(d["trading_fee"], "0");
            assert_eq!(parse(&d["wrapper_cost"]), gross - net);
        }
    }

    #[test]
    fn issue1500_pure_book_commission_sell_buy_tax_and_hook_loss_have_zero_order_movement() {
        use super::super::tax::TokenObservation;
        use crate::api::local_hook_pricing::CachedHookPrices;
        let mut s = fixtures::fixture();
        s.quote_relative_protection = true;
        let input = 10u128.pow(18);
        let m = Arc::make_mut(s.mirrors.get_mut("pair7a").unwrap());
        m.asset_1_addr = "out18".into();
        m.reserve_0 = 10 * input;
        m.reserve_1 = 10 * input;
        m.bids
            .push(crate::db::queries::resting_orders::RestingOrderRow {
                pair_id: m.pair_id,
                order_id: 1,
                side: "bid".into(),
                price: 1.into(),
                remaining: (10 * input).into(),
                owner: None,
                expires_at: None,
            });
        for (token, buy, sell) in [("in18", 0, 250), ("out18", 333, 0)] {
            s.taxes.insert(
                token.into(),
                TokenObservation {
                    policy_identity: "owned-order-fixture".into(),
                    code_id: 1,
                    buy_bps: buy,
                    sell_bps: sell,
                    observed_at_ms: super::super::now_ms(),
                    max_wallet: None,
                    pairs: ["pair7a".into()].into(),
                    actors: Default::default(),
                },
            );
        }
        s.hooks.insert(
            "pair7a".into(),
            CachedHookPrices::from_observation(&[(
                "owned".into(),
                json!({"burn_token":"out18","burn_percentage_bps":100}),
            )])
            .unwrap(),
        );
        let c = score_with_book(
            &s,
            &vec![("pair7a".into(), 1, 2)],
            input,
            SCALE / 20,
            input,
            8,
        )
        .unwrap();
        let body = response(&s, &fixtures::request(input), &c, input, SCALE / 20, 1);
        let d = &body["order_economics"];
        assert_eq!(d["status"], "available");
        assert_eq!(d["order_movement"], "0");
        assert_eq!(parse(&d["baseline_amount_out"]), input);
        assert_eq!(parse(&d["fee_free_amount_out"]), input);
        assert!(parse(&d["trading_fee"]) > 0);
        assert!(parse(&d["tax_hook_cost"]) > 0);
        // Actual zero-size book commission is per-maker taker bps (30 - floor(30/2)).
        let economic = ((input + 1) * 10000 - 1) / 10250;
        let post_fee = economic - economic * 15 / 10000;
        let post_hook = post_fee - post_fee * 100 / 10000;
        assert_eq!(c.output, post_hook - post_hook * 333 / 10000);
    }
    #[test]
    fn issue1500_pool_counterfactual_is_independent_of_actual_fees_and_reference() {
        let mut s = fixtures::fixture();
        s.quote_relative_protection = true;
        let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
        let amount = 10u128.pow(18);
        // Independently apply constant product twice WITHOUT either commission.
        let first =
            amount * 1_000_000u128 * 10u128.pow(6) / (1_000_000u128 * 10u128.pow(18) + amount);
        let expected: u128 = (BigInt::from(first) * BigInt::from(1_000_000u128 * 10u128.pow(18))
            / BigInt::from(1_000_000u128 * 10u128.pow(6) + first))
        .to_string()
        .parse()
        .unwrap();
        for bps in [0, 30, 400] {
            for pair in ["pair7a", "pair7b"] {
                Arc::make_mut(s.mirrors.get_mut(pair).unwrap()).fee_bps = bps;
            }
            let c = score(&s, &path, amount, SCALE / 20).unwrap();
            let body = response(&s, &fixtures::request(amount), &c, amount, SCALE / 20, 1);
            let d = &body["order_economics"];
            assert_eq!(d["status"], "available");
            assert_eq!(parse(&d["baseline_amount_out"]), amount);
            assert_eq!(parse(&d["fee_free_amount_out"]), expected);
            assert_eq!(parse(&d["order_movement"]), amount - expected);
            assert_eq!(d["router_operations"], body["router_operations"]);
            assert_eq!(d["generation"], body["execution_metadata"]["generation"]);
            if bps == 0 {
                assert_eq!(d["trading_fee"], "0");
            } else {
                assert!(parse(&d["trading_fee"]) > 0);
            }
        }
    }

    #[test]
    fn issue1500_favorable_book_prices_and_residue_preserve_signed_slippage() {
        let mut s = fixtures::fixture();
        s.quote_relative_protection = true;
        let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
        let mirror = Arc::make_mut(s.mirrors.get_mut("pair7a").unwrap());
        mirror
            .bids
            .push(crate::db::queries::resting_orders::RestingOrderRow {
                pair_id: mirror.pair_id,
                order_id: 1,
                side: "bid".into(),
                price: "0.000000000002".parse().unwrap(),
                remaining: 10_000_000.into(),
                owner: None,
                expires_at: None,
            });
        // Original input exhausts its budget on maker one. Retain maker two
        // nonetheless: removing upstream losses can reach previously unused rows.
        mirror
            .bids
            .push(crate::db::queries::resting_orders::RestingOrderRow {
                pair_id: mirror.pair_id,
                order_id: 2,
                side: "bid".into(),
                price: "0.000000000003".parse().unwrap(),
                remaining: 10_000_000.into(),
                owner: None,
                expires_at: None,
            });
        let amount = 10u128.pow(18);
        for budget in [amount / 2, amount] {
            let c = score_with_book(&s, &path, amount, SCALE / 20, budget, 8).unwrap();
            let body = response(&s, &fixtures::request(amount), &c, amount, SCALE / 20, 1);
            let d = &body["order_economics"];
            assert_eq!(d["status"], "available");
            assert!(d["order_movement"].as_str().unwrap().starts_with('-'));
            let original_terms = &body["execution_metadata"]["hops"][0]["book_terms"];
            assert_eq!(original_terms.as_array().unwrap().len(), 2);
            assert_eq!(original_terms[1]["price_atomics"], "3000000");
            assert_eq!(d["stages"][0]["book"], *original_terms);
            // Maker price 2e-12 raw ask per offer; mixed remainder uses original CP.
            let pool = amount - budget;
            let first = budget * 2 / 10u128.pow(12)
                + pool * 1_000_000u128 * 10u128.pow(6) / (1_000_000u128 * 10u128.pow(18) + pool);
            let second: u128 = (BigInt::from(first) * BigInt::from(1_000_000u128 * 10u128.pow(18))
                / BigInt::from(1_000_000u128 * 10u128.pow(6) + first))
            .to_string()
            .parse()
            .unwrap();
            assert_eq!(parse(&d["fee_free_amount_out"]), second);
        }
    }

    #[test]
    fn issue1500_missing_projection_is_unavailable_without_changing_quote() {
        let mut s = fixtures::fixture();
        s.quote_relative_protection = true;
        let amount = 10u128.pow(18);
        let mut c = score(
            &s,
            &vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)],
            amount,
            SCALE / 20,
        )
        .unwrap();
        let before = response(&s, &fixtures::request(amount), &c, amount, SCALE / 20, 1);
        c.economics[0].reserve_in = 0;
        let after = response(&s, &fixtures::request(amount), &c, amount, SCALE / 20, 1);
        assert_eq!(after["order_economics"]["status"], "unavailable");
        for key in [
            "estimated_amount_out",
            "spot_amount_out",
            "execution_metadata",
            "router_operations",
        ] {
            assert_eq!(before[key], after[key]);
        }
    }
}
