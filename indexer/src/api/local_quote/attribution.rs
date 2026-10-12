//! Exact common-output economic attribution (#1483).
//!
//! This is a diagnostic spot bridge, not an oracle or a counterfactual executable
//! quote. Each stage is projected through the original remaining reserve ratios
//! with one integer floor per remaining hop. Differences of projected boundaries
//! telescope exactly; integer truncation belongs to the stage where it occurs.
//! No percentages are added and no new simulation, observation or RPC is performed.
use super::{wide_floor, BigInt, Candidate, Value};
use serde_json::json;

const METHOD: &str = "spot_bridge_v1";
const NAMES: [&str; 4] = [
    "wrapper_loss",
    "trading_cost",
    "market_movement",
    "reference_divergence",
];

#[derive(Clone)]
pub(super) struct HopEconomics {
    pub reserve_in: u128,
    pub reserve_out: u128,
    pub input: u128,
    pub economic_input: u128,
    // Sum may exceed Uint128 even when the executable net does not. In that
    // case attribution is unavailable without changing executable admission.
    pub pre_commission: Option<u128>,
    pub post_commission: u128,
    pub post_hook: u128,
    pub net: u128,
}

fn project(mut amount: u128, suffix: &[HopEconomics]) -> Option<u128> {
    for hop in suffix {
        if hop.reserve_in == 0 || hop.reserve_out == 0 {
            return None;
        }
        amount = wide_floor(amount, hop.reserve_out, hop.reserve_in)?;
    }
    Some(amount)
}

fn difference(a: u128, b: u128) -> BigInt {
    BigInt::from(a) - BigInt::from(b)
}

fn magnitude(value: &BigInt) -> BigInt {
    if value < &BigInt::from(0) {
        -value
    } else {
        value.clone()
    }
}

fn components(
    hops: &[HopEconomics],
    gross: u128,
    credit: u128,
    route_net: u128,
    wallet_net: u128,
    fair: u128,
) -> Option<[BigInt; 4]> {
    if hops.is_empty()
        || hops.len() > 4
        || hops[0].input != credit
        || credit > gross
        || wallet_net > route_net
    {
        return None;
    }
    let gross_spot = project(gross, hops)?;
    let credit_spot = project(credit, hops)?;
    let mut result = [
        difference(gross_spot, credit_spot) + difference(route_net, wallet_net),
        BigInt::from(0),
        BigInt::from(0),
        difference(fair, gross_spot),
    ];
    let mut previous = credit;
    for (index, hop) in hops.iter().enumerate() {
        let pre_commission = hop.pre_commission?;
        if hop.input != previous
            || hop.economic_input > hop.input
            || hop.post_commission > pre_commission
            || hop.post_hook > hop.post_commission
            || hop.net > hop.post_hook
        {
            return None;
        }
        let ideal_gross = project(hop.input, &hops[index..])?;
        let ideal_economic = project(hop.economic_input, &hops[index..])?;
        let suffix = &hops[index + 1..];
        let before_fee = project(pre_commission, suffix)?;
        let after_fee = project(hop.post_commission, suffix)?;
        let after_hook = project(hop.post_hook, suffix)?;
        let net = project(hop.net, suffix)?;
        // Sell funding loss includes tax plus unspent integer residue. The
        // latter is not delivered ask output and remains a separate hop field.
        result[1] += difference(ideal_gross, ideal_economic)
            + difference(before_fee, after_fee)
            + difference(after_fee, after_hook)
            + difference(after_hook, net);
        // Signed: favorable book pricing may improve on the AMM spot bridge.
        result[2] += difference(ideal_economic, before_fee);
        previous = hop.net;
    }
    if previous != route_net
        || result.iter().cloned().sum::<BigInt>() != difference(fair, wallet_net)
        || result
            .iter()
            .any(|amount| magnitude(amount) > BigInt::from(u128::MAX))
    {
        return None;
    }
    Some(result)
}

pub(super) fn unavailable() -> Value {
    json!({"version":1,"status":"unavailable","method":METHOD})
}

fn available(parts: [BigInt; 4], fair: u128, net: u128, body: &Value) -> Option<Value> {
    let meta = &body["execution_metadata"];
    let generation = meta["generation"].as_str()?;
    let observed = meta["observed_at_ms"].as_i64()?;
    let expires = meta["expires_at_ms"].as_i64()?;
    if generation.is_empty() || generation.len() > 128 || expires <= observed {
        return None;
    }
    let max = parts.iter().map(magnitude).max()?;
    let mut greatest = parts
        .iter()
        .enumerate()
        .filter(|(_, amount)| magnitude(amount) == max);
    let (first, _) = greatest.next()?;
    let dominant = if greatest.next().is_none() {
        NAMES[first]
    } else {
        "mixed"
    };
    let mut amounts = serde_json::Map::new();
    for (name, amount) in NAMES.iter().zip(parts) {
        amounts.insert((*name).to_owned(), json!(amount.to_string()));
    }
    Some(json!({
        "version":1,"status":"available","method":METHOD,
        "fair_amount_out":fair.to_string(),"net_amount_out":net.to_string(),
        "generation":generation,"observed_at_ms":observed,"expires_at_ms":expires,
        "components":amounts,"dominant_cause":dominant
    }))
}

pub(super) fn route(
    candidate: &Candidate,
    gross: u128,
    credit: u128,
    wallet_net: u128,
    body: &Value,
) -> Value {
    let result = (|| {
        let fair = body["spot_amount_out"].as_str()?.parse().ok()?;
        let parts = components(
            &candidate.economics,
            gross,
            credit,
            candidate.output,
            wallet_net,
            fair,
        )?;
        available(parts, fair, wallet_net, body)
    })();
    result.unwrap_or_else(unavailable)
}

pub(super) fn wrapper_only(gross: u128, fair: u128, net: u128, body: &Value) -> Value {
    available(
        [
            difference(gross, net),
            BigInt::from(0),
            BigInt::from(0),
            difference(fair, gross),
        ],
        fair,
        net,
        body,
    )
    .unwrap_or_else(unavailable)
}

#[cfg(test)]
mod tests {
    use super::super::{final_response, response, score, tests as fixtures, SCALE};
    use super::*;

    fn hop(input: u128, net: u128, ratio: u128) -> HopEconomics {
        HopEconomics {
            reserve_in: 1,
            reserve_out: ratio,
            input,
            economic_input: input,
            pre_commission: Some(net),
            post_commission: net,
            post_hook: net,
            net,
        }
    }

    fn binding(fair: u128) -> Value {
        json!({"spot_amount_out":fair.to_string(),"execution_metadata":{
            "generation":"original-generation","observed_at_ms":100,"expires_at_ms":20_100
        }})
    }

    fn assert_reconciled(body: &Value) {
        let attr = &body["economic_attribution"];
        assert_eq!(attr["status"], "available");
        assert_eq!(attr["fair_amount_out"], body["spot_amount_out"]);
        assert_eq!(attr["net_amount_out"], body["estimated_amount_out_net"]);
        let sum: BigInt = NAMES
            .iter()
            .map(|name| {
                attr["components"][name]
                    .as_str()
                    .unwrap()
                    .parse::<BigInt>()
                    .unwrap()
            })
            .sum();
        assert_eq!(
            sum,
            attr["fair_amount_out"]
                .as_str()
                .unwrap()
                .parse::<BigInt>()
                .unwrap()
                - attr["net_amount_out"]
                    .as_str()
                    .unwrap()
                    .parse::<BigInt>()
                    .unwrap()
        );
        for key in ["generation", "observed_at_ms", "expires_at_ms"] {
            assert_eq!(attr[key], body["execution_metadata"][key]);
        }
        assert!(serde_json::to_vec(attr).unwrap().len() <= 1_024);
    }

    #[test]
    fn exact_raw_attribution_reconciles_wrappers_compounded_costs_and_movement() {
        // Independent hand-worked fixture, final units throughout:
        // gross spot 100*2*3=600, credit spot=588, independent fair=650.
        // First-hop costs (588-540)+(450-441)+(441-420)+(420-411)=87;
        // second-hop costs (411-390)+(300-290)+(290-285)+(285-280)=41.
        // Movement=(540-450)+(390-300)=180; wrappers=12+(280-260)=32.
        let hops = [
            HopEconomics {
                reserve_in: 1,
                reserve_out: 2,
                input: 98,
                economic_input: 90,
                pre_commission: Some(150),
                post_commission: 147,
                post_hook: 140,
                net: 137,
            },
            HopEconomics {
                reserve_in: 1,
                reserve_out: 3,
                input: 137,
                economic_input: 130,
                pre_commission: Some(300),
                post_commission: 290,
                post_hook: 285,
                net: 280,
            },
        ];
        let parts = components(&hops, 100, 98, 280, 260, 650).unwrap();
        assert_eq!(parts.map(|n| n.to_string()), ["32", "128", "180", "50"]);
    }

    #[test]
    fn favorable_book_and_reference_offsets_remain_signed_not_zero_impact() {
        let parts = components(&[hop(100, 125, 1)], 100, 100, 125, 125, 90).unwrap();
        assert_eq!(
            parts.clone().map(|n| n.to_string()),
            ["0", "0", "-25", "-10"]
        );
        let attr = available(parts, 90, 125, &binding(90)).unwrap();
        assert_eq!(attr["dominant_cause"], "market_movement");
        let offset = components(&[hop(100, 80, 1)], 100, 100, 80, 80, 90).unwrap();
        assert_eq!(offset.map(|n| n.to_string()), ["0", "0", "20", "-10"]);
    }

    #[test]
    fn stage_rounding_and_mixed_precision_telescope_with_zero_raw_residual() {
        // Floors across 6 -> 18 -> 6 scales. Independent raw values imply
        // fee-free benchmark=floor(floor(101*10^12)*7/(3*10^12))=235.
        let mut first = hop(101, 99 * 10u128.pow(12) + 1, 10u128.pow(12));
        first.economic_input = 100;
        let second = HopEconomics {
            reserve_in: 3 * 10u128.pow(12),
            reserve_out: 7,
            input: first.net,
            economic_input: first.net,
            pre_commission: Some(225),
            post_commission: 224,
            post_hook: 222,
            net: 221,
        };
        let parts = components(&[first, second], 103, 101, 221, 220, 250).unwrap();
        assert_eq!(parts.map(|n| n.to_string()), ["6", "6", "8", "10"]);
    }

    #[test]
    fn unavailable_evidence_and_uint128_projection_never_fabricate_zero_costs() {
        assert!(components(&[], 100, 100, 90, 90, 100).is_none());
        assert!(components(&vec![hop(100, 100, 1); 5], 100, 100, 100, 100, 100).is_none());
        for case in 0..6 {
            let mut broken = hop(100, 90, 1);
            match case {
                0 => broken.reserve_in = 0,
                1 => broken.reserve_out = 0,
                2 => broken.economic_input = 101,
                3 => broken.pre_commission = Some(89),
                4 => broken.post_hook = 91,
                _ => broken.post_hook = 89,
            }
            assert!(components(&[broken], 100, 100, 90, 90, 100).is_none());
        }
        let mut broken = hop(100, 90, 1);
        broken.pre_commission = None;
        assert!(components(&[broken], 100, 100, 90, 90, 100).is_none());
        assert!(components(
            &[hop(u128::MAX, u128::MAX, 2)],
            u128::MAX,
            u128::MAX,
            u128::MAX,
            u128::MAX,
            u128::MAX
        )
        .is_none());
        assert!(components(&[hop(100, 90, 1), hop(91, 80, 1)], 100, 100, 80, 80, 100).is_none());
        assert!(components(&[hop(100, 90, 1)], 99, 100, 90, 90, 100).is_none());
        assert!(components(&[hop(100, 90, 1)], 100, 100, 90, 91, 100).is_none());
        let full = components(
            &[hop(u128::MAX, u128::MAX, 1)],
            u128::MAX,
            u128::MAX,
            u128::MAX,
            u128::MAX,
            u128::MAX,
        )
        .unwrap();
        assert_eq!(full, std::array::from_fn(|_| BigInt::from(0)));
        assert_eq!(
            unavailable(),
            json!({"version":1,"status":"unavailable","method":"spot_bridge_v1"})
        );
    }

    #[test]
    fn ties_binding_failures_and_zero_hop_wrappers_are_explicit() {
        let attr = available([0, 4, 4, 0].map(BigInt::from), 100, 92, &binding(100)).unwrap();
        assert_eq!(attr["dominant_cause"], "mixed");
        let mut invalid = binding(100);
        invalid["execution_metadata"]["expires_at_ms"] = json!(100);
        assert!(available([0, 4, 4, 0].map(BigInt::from), 100, 92, &invalid).is_none());
        let direct = wrapper_only(100, 100, 98, &binding(100));
        assert_eq!(direct["dominant_cause"], "wrapper_loss");
        assert_eq!(direct["components"]["wrapper_loss"], "2");
    }

    #[test]
    fn selected_quote_attribution_preserves_independent_economics_and_floors() {
        let mut s = fixtures::fixture();
        s.quote_relative_protection = true;
        let input = 10u128.pow(18);
        let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
        let c = score(&s, &path, input, SCALE / 20).unwrap();
        // Independently evaluate constant product and floor each commission,
        // rather than deriving the expected output from attribution helpers.
        let first_gross =
            input * 1_000_000u128 * 10u128.pow(6) / (1_000_000u128 * 10u128.pow(18) + input);
        let first_net = first_gross - first_gross * 30 / 10_000;
        let second_gross = BigInt::from(first_net) * BigInt::from(1_000_000u128 * 10u128.pow(18))
            / BigInt::from(1_000_000u128 * 10u128.pow(6) + first_net);
        let second_gross: u128 = second_gross.to_string().parse().unwrap();
        let expected = second_gross - second_gross * 30 / 10_000;
        assert_eq!(c.output, expected);
        let body = response(&s, &fixtures::request(input), &c, input, SCALE / 20, 1);
        assert_reconciled(&body);
        assert_eq!(
            body["economic_attribution"]["dominant_cause"],
            "trading_cost"
        );
        assert_eq!(
            body["execution_metadata"]["minimum_receive"],
            (expected - expected * 5 / 100).to_string()
        );
        let mut unavailable = c.clone();
        unavailable.economics[0].reserve_in = 0;
        let failed = response(
            &s,
            &fixtures::request(input),
            &unavailable,
            input,
            SCALE / 20,
            1,
        );
        assert_eq!(failed["economic_attribution"]["status"], "unavailable");
        for field in [
            "estimated_amount_out_net",
            "spot_amount_out",
            "slippage_percent",
            "router_operations",
            "execution_metadata",
        ] {
            assert_eq!(failed[field], body[field]);
        }
        assert!(final_response(
            &s,
            &fixtures::request(input),
            &c,
            input,
            SCALE / 20,
            1,
            None
        )
        .is_some());
    }

    #[test]
    fn observed_sell_buy_taxes_and_individually_floored_hooks_reconcile() {
        use super::super::tax::TokenObservation;
        use crate::api::local_hook_pricing::CachedHookPrices;
        use std::collections::BTreeMap;
        let mut s = fixtures::fixture();
        s.quote_relative_protection = true;
        s.taxes.insert(
            "middle76".into(),
            TokenObservation {
                policy_identity: "owned-tax-fixture".into(),
                code_id: 1,
                buy_bps: 333,
                sell_bps: 250,
                observed_at_ms: super::super::now_ms(),
                max_wallet: None,
                pairs: ["pair7a".into(), "pair7b".into()].into(),
                actors: BTreeMap::new(),
            },
        );
        s.hooks.insert(
            "pair7b".into(),
            CachedHookPrices::from_observation(&[
                (
                    "one".into(),
                    json!({"burn_token":"out18","burn_percentage_bps":333}),
                ),
                (
                    "two".into(),
                    json!({"burn_token":"out18","burn_percentage_bps":250}),
                ),
            ])
            .unwrap(),
        );
        let input = 10u128.pow(18);
        let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
        let c = score(&s, &path, input, SCALE / 20).unwrap();
        let first_gross =
            input * 1_000_000u128 * 10u128.pow(6) / (1_000_000u128 * 10u128.pow(18) + input);
        let first_commission_net = first_gross - first_gross * 30 / 10_000;
        let first_net = first_commission_net - first_commission_net * 333 / 10_000;
        // The greatest economic e satisfying e+floor(e*250/10000)<=first_net
        // is obtained independently by this closed integer inequality.
        let economic = ((first_net + 1) * 10_000 - 1) / 10_250;
        let second_gross = BigInt::from(economic) * BigInt::from(1_000_000u128 * 10u128.pow(18))
            / BigInt::from(1_000_000u128 * 10u128.pow(6) + economic);
        let second_gross: u128 = second_gross.to_string().parse().unwrap();
        let post_commission = second_gross - second_gross * 30 / 10_000;
        let expected =
            post_commission - post_commission * 333 / 10_000 - post_commission * 250 / 10_000;
        assert_eq!(c.output, expected);
        assert_eq!(c.hops[1]["economic_offer_amount"], economic.to_string());
        assert_eq!(c.hops[1]["post_hook_return_amount"], expected.to_string());
        let body = response(&s, &fixtures::request(input), &c, input, SCALE / 20, 1);
        assert_reconciled(&body);
        assert_eq!(
            body["economic_attribution"]["dominant_cause"],
            "trading_cost"
        );
        assert_eq!(body["tax_funding_version"], 1);
    }

    #[test]
    fn one_to_four_hop_integer_boundary_corpus_reconciles_exactly() {
        for amount in [
            1,
            2,
            3,
            9,
            10,
            11,
            99,
            100,
            101,
            1_000,
            4_000,
            10_000,
            u64::MAX as u128,
        ] {
            for count in 1..=4 {
                let mut previous = amount;
                let mut hops = Vec::new();
                for _ in 0..count {
                    // Exact boundary cost e.g. one raw commission unit and
                    // nonintegral spot projection; no floating-point arithmetic.
                    let ideal = previous * 7 / 3;
                    let net = ideal.saturating_sub(1);
                    let mut next = hop(previous, net, 7);
                    next.reserve_in = 3;
                    next.pre_commission = Some(ideal);
                    hops.push(next);
                    previous = net;
                }
                let fair = amount * 30;
                let parts = components(&hops, amount, amount, previous, previous, fair).unwrap();
                assert_eq!(
                    parts.iter().cloned().sum::<BigInt>(),
                    difference(fair, previous)
                );
            }
        }
    }
}
