//! Bounded exact-output pool sizing from immutable observations (#1459).
//!
//! Individual hook floors can lower delivered output when gross output increases.
//! A backward composition therefore gives a necessary lower bound, not a complete
//! inverse. Check the whole route and refine at the next first-hop pool-output
//! event. Pool output *before hooks* is monotone; delivered intermediate output is
//! not. Advancing an intermediate net threshold would skip valid upstream dips.
use super::*;
use crate::api::local_hook_pricing::CachedHookPrices;

const MAX_INVERSE_PATHS: usize = 64;
const MAX_FORWARD_REFINEMENTS: usize = 64;
const MAX_HOP_REFINEMENTS: usize = 64;
const MAX_INVERSE_PROBES: usize = 512;

struct WorkBudget {
    remaining: usize,
}
impl WorkBudget {
    fn probe(&mut self) -> Option<()> {
        self.remaining = self.remaining.checked_sub(1)?;
        Some(())
    }
}

struct PoolLeg<'a> {
    mirror: &'a sim::HopMirror,
    offer: &'a str,
    ask: &'a str,
    hooks: Option<&'a CachedHookPrices>,
    sell_bps: u16,
    buy_bps: u16,
    reserve_in: u128,
    reserve_out: u128,
}
impl<'a> PoolLeg<'a> {
    fn new(s: &'a Snapshot, (pair, offer, ask): &route_paths::PathHop) -> Option<Self> {
        let offer = s.addresses.get(offer)?.as_str();
        let ask = s.addresses.get(ask)?.as_str();
        let mirror = s.mirrors.get(pair)?;
        let (reserve_in, reserve_out) =
            if offer == mirror.asset_0_addr && ask == mirror.asset_1_addr {
                (mirror.reserve_0, mirror.reserve_1)
            } else if offer == mirror.asset_1_addr && ask == mirror.asset_0_addr {
                (mirror.reserve_1, mirror.reserve_0)
            } else {
                return None;
            };
        Some(Self {
            mirror,
            offer,
            ask,
            hooks: s.hooks.get(pair),
            sell_bps: s.taxes.get(offer).map_or(Some(0), |tax| {
                tax.rates(s.tax_trader.as_deref(), pair).map(|r| r.0)
            })?,
            buy_bps: s.taxes.get(ask).map_or(Some(0), |tax| {
                tax.rates(s.tax_trader.as_deref(), pair).map(|r| r.1)
            })?,
            reserve_in,
            reserve_out,
        })
    }

    fn original(&self, economic: u128) -> Option<u128> {
        Some(
            sim::quote_local_leg(self.mirror, self.offer, economic, 0, 8)
                .ok()?
                .return_amount,
        )
    }

    fn delivered(&self, original: u128) -> Option<u128> {
        let post_hook = self.hooks.map_or(Some(original), |hooks| {
            hooks.net_return(self.ask, original).ok()
        })?;
        Some(tax::buy(post_hook, self.buy_bps)?.0)
    }

    fn forward(&self, gross: u128) -> Option<(u128, u128)> {
        let original = self.original(tax::sell(gross, self.sell_bps)?.0)?;
        Some((self.delivered(original)?, original))
    }

    /// First gross offer delivering at least target, constrained only by a
    /// monotone pre-hook pool-output lower bound. Reachability jumps must be
    /// checked against the actual hooks rather than a monotone approximation.
    fn inverse(&self, target: u128, mut lower: u128, work: &mut WorkBudget) -> Option<u128> {
        let post_hook_target = tax::inverse_buy(target, self.buy_bps)?;
        for _ in 0..MAX_HOP_REFINEMENTS {
            work.probe()?;
            let original_target = self.hooks.map_or(Some(post_hook_target.max(lower)), |h| {
                h.inverse_return_from(self.ask, post_hook_target, lower)
                    .ok()
            })?;
            let economic = inverse_pool_offer(
                self.reserve_in,
                self.reserve_out,
                self.mirror.fee_bps,
                original_target,
            )?;
            let original = self.original(economic)?;
            if self.delivered(original)? >= target {
                return economic.checked_add(super::super::community_tax_rank::tax_amount(
                    economic,
                    self.sell_bps,
                ));
            }
            lower = original.checked_add(1)?;
        }
        None
    }
}

fn inverse_pool_offer(rin: u128, rout: u128, fee_bps: u16, wanted: u128) -> Option<u128> {
    if rin == 0 || wanted == 0 || fee_bps >= 10_000 || wanted >= rout {
        return None;
    }
    // Net(gross) = ceil(gross * retained_bps / 10_000). Invert that
    // inequality exactly, including independent fee rounding, without a search.
    let gross = ((BigInt::from(wanted - 1) * BigInt::from(10_000u16))
        / BigInt::from(10_000 - fee_bps)
        + BigInt::from(1u8))
    .to_string()
    .parse::<u128>()
    .ok()?;
    let denominator = BigInt::from(rout.checked_sub(gross)?);
    if denominator == BigInt::from(0u8) {
        return None;
    }
    let numerator = BigInt::from(rin) * BigInt::from(gross);
    ((numerator + &denominator - BigInt::from(1u8)) / denominator)
        .to_string()
        .parse::<u128>()
        .ok()
}

fn compose_inverse(
    legs: &[PoolLeg<'_>],
    target: u128,
    first_original_lower: u128,
    work: &mut WorkBudget,
) -> Option<u128> {
    let mut wanted = target;
    for (index, leg) in legs.iter().enumerate().rev() {
        wanted = leg.inverse(
            wanted,
            if index == 0 { first_original_lower } else { 0 },
            work,
        )?;
    }
    Some(wanted)
}

#[cfg(test)]
pub(super) fn inverse_pool_path(
    s: &Snapshot,
    path: &[route_paths::PathHop],
    target: u128,
) -> Option<u128> {
    let legs = path
        .iter()
        .map(|hop| PoolLeg::new(s, hop))
        .collect::<Option<Vec<_>>>()?;
    compose_inverse(
        &legs,
        target,
        0,
        &mut WorkBudget {
            remaining: MAX_INVERSE_PROBES,
        },
    )
}

fn inverse_candidate(
    s: &Snapshot,
    path: &[route_paths::PathHop],
    target: u128,
    spread: u128,
    work: &mut WorkBudget,
) -> Option<(u128, Candidate)> {
    if path.is_empty() || target == 0 {
        return None;
    }
    let legs = path
        .iter()
        .map(|hop| PoolLeg::new(s, hop))
        .collect::<Option<Vec<_>>>()?;
    let mut lower = 0;
    for _ in 0..MAX_FORWARD_REFINEMENTS {
        let required = compose_inverse(&legs, target, lower, work)?;
        let (mut output, original) = legs[0].forward(required)?;
        for leg in &legs[1..] {
            output = leg.forward(output)?.0;
        }
        if output >= target {
            // Preserve all admission, wallet, L9 and fair-reference protections.
            let candidate = score(s, path, required, spread)?;
            return (candidate.output >= target).then_some((required, candidate));
        }
        // At a fixed first-hop original pool output the entire route is fixed.
        // Skip that plateau, but preserve subsequent upstream hook dips. Each
        // backward candidate is a necessary lower bound; together these facts
        // establish minimality when the full forward result first reaches target.
        lower = original.checked_add(1)?;
    }
    None
}

pub(super) fn select_inverse(
    s: &Snapshot,
    paths: &[Vec<route_paths::PathHop>],
    target: u128,
    spread: u128,
) -> Option<(u128, Candidate)> {
    let mut best = None;
    for path in paths.iter().take(MAX_INVERSE_PATHS) {
        let mut work = WorkBudget {
            remaining: MAX_INVERSE_PROBES,
        };
        if let Some((required, candidate)) = inverse_candidate(s, path, target, spread, &mut work) {
            if best
                .as_ref()
                .is_none_or(|(amount, _): &(u128, Candidate)| required < *amount)
            {
                best = Some((required, candidate));
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hooks(ask: &str, fees: &[u16]) -> CachedHookPrices {
        CachedHookPrices::from_observation(
            &fees
                .iter()
                .enumerate()
                .map(|(i, fee)| {
                    (
                        format!("deduction{i}"),
                        json!({"burn_token":ask,"burn_percentage_bps":fee}),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .unwrap()
    }

    // Independent small-integer evaluator. It deliberately does not invoke the
    // quote simulator, inverse helpers, hook tables or scorer. The tuple binds
    // reserve orientation, commission and independent hook floors per hop.
    fn integer_route(mut input: u128, legs: &[(u128, u128, u16, &[u16])]) -> u128 {
        for (rin, rout, commission_bps, fees) in legs {
            let pool_gross = rout * input / (rin + input);
            let original = pool_gross - pool_gross * u128::from(*commission_bps) / 10_000;
            input = original
                - fees
                    .iter()
                    .map(|bps| original * u128::from(*bps) / 10_000)
                    .sum::<u128>();
        }
        input
    }

    fn two_hop(
        first_reserves: (u128, u128),
        first_fees: &[u16],
    ) -> (Snapshot, Vec<route_paths::PathHop>) {
        let mut s = super::super::tests::fixture();
        let first = s.mirrors.get_mut("pair7a").unwrap();
        first.reserve_0 = first_reserves.0;
        first.reserve_1 = first_reserves.1;
        first.fee_bps = 0;
        let second = s.mirrors.get_mut("pair7b").unwrap();
        second.reserve_0 = 1000;
        second.reserve_1 = 1002;
        second.fee_bps = 0;
        s.hooks
            .insert("pair7a".into(), hooks("middle76", first_fees));
        s.hooks
            .insert("pair7b".into(), hooks("out18", &[2500, 2500]));
        (s, vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)])
    }

    #[test]
    fn composed_inverse_keeps_upstream_hook_dips_instead_of_advancing_net_threshold() {
        let (s, path) = two_hop((16, 35), &[2500, 2500]);
        let legs = [
            (16, 35, 0, &[2500, 2500][..]),
            (1000, 1002, 0, &[2500, 2500][..]),
        ];
        // First necessary input4 reaches upstream original7/net5. Downstream
        // original4 lands in a dip. Input5 reaches upstream original8/net4;
        // that *smaller* intermediate amount delivers the required final3.
        assert_eq!(integer_route(4, &legs), 2);
        assert_eq!(integer_route(5, &legs), 3);
        let first = score(&s, &path, 4, SCALE / 2).unwrap();
        let (required, quote) = select_inverse(&s, &[path.clone()], 3, SCALE / 2).unwrap();
        assert_eq!(required, 5);
        assert_eq!(quote.output, 3);
        assert_eq!(first.hops[0]["return_amount"], "5");
        assert_eq!(quote.hops[0]["return_amount"], "4");
        assert!((0..required).all(|input| integer_route(input, &legs) < 3));
    }

    #[test]
    fn zero_delivered_collisions_refine_before_final_admission_and_positive_floors() {
        let (mut s, path) = two_hop((1000, 2500), &[]);
        s.hooks
            .insert("pair7b".into(), hooks("out18", &[5000, 5000]));
        let legs = [(1000, 2500, 0, &[][..]), (1000, 1002, 0, &[5000, 5000][..])];
        assert_eq!(integer_route(1, &legs), 0);
        assert!(score(&s, &path, 1, SCALE / 2).is_none());
        let (required, quote) = select_inverse(&s, &[path.clone()], 1, SCALE / 2).unwrap();
        assert_eq!(required, 2);
        assert_eq!(quote.output, integer_route(required, &legs));
        assert!((0..required).all(|input| integer_route(input, &legs) < 1));
        assert!(quote
            .operations
            .iter()
            .all(|op| op["terra_swap"]["min_return"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                > 0));
    }

    #[test]
    fn composed_inverse_matches_independent_first_reaching_census_with_commissions_and_three_hops()
    {
        let (mut s, mut path) = two_hop((1000, 6000), &[3333, 3333]);
        s.mirrors.get_mut("pair7a").unwrap().fee_bps = 333;
        s.mirrors.get_mut("pair7b").unwrap().fee_bps = 250;
        let mut final_mirror = s.mirrors["pair7b"].clone();
        final_mirror.asset_0_addr = "out18".into();
        final_mirror.asset_1_addr = "end18".into();
        final_mirror.reserve_0 = 2000;
        final_mirror.reserve_1 = 2200;
        final_mirror.fee_bps = 125;
        s.addresses.insert(20, "end18".into());
        s.references = super::super::super::local_reference_quote::ReferencePrices::synthetic(
            s.addresses.keys().copied(),
        );
        s.mirrors.insert("third".into(), final_mirror);
        s.hooks
            .insert("third".into(), hooks("end18", &[1000, 1500]));
        path.push(("third".into(), 2, 20));
        let legs = [
            (1000, 6000, 333, &[3333, 3333][..]),
            (1000, 1002, 250, &[2500, 2500][..]),
            (2000, 2200, 125, &[1000, 1500][..]),
        ];
        for target in 1..=40 {
            let expected = (1..=200)
                .find(|input| integer_route(*input, &legs) >= target)
                .unwrap();
            let (required, quote) = select_inverse(&s, &[path.clone()], target, SCALE / 2).unwrap();
            assert_eq!(required, expected, "target {target}");
            assert_eq!(quote.output, integer_route(required, &legs));
            assert!(quote.output >= target);
        }
    }

    #[test]
    fn exhausted_shared_work_never_publishes_insufficient_composition() {
        let (s, path) = two_hop((1000, 6000), &[]);
        let mut work = WorkBudget { remaining: 2 };
        assert!(inverse_candidate(&s, &path, 3, SCALE / 2, &mut work).is_none());
        assert_eq!(work.remaining, 0);
        assert!(work.probe().is_none());
        assert_eq!(select_inverse(&s, &[path], 3, SCALE / 2).unwrap().0, 2);
    }

    #[test]
    fn forward_refinement_limit_fails_closed_and_keeps_a_sufficient_alternate() {
        let (mut s, path) = two_hop((1, 4001), &[]);
        s.mirrors.get_mut("pair7b").unwrap().reserve_0 = 3997;
        s.mirrors.get_mut("pair7b").unwrap().reserve_1 = 6;
        s.hooks
            .insert("pair7b".into(), hooks("out18", &[5000, 5000]));
        // An early necessary suffix input is overshot into a long even-output
        // plateau. All 64 first-hop events fail, but eventual input1000 reaches
        // an odd downstream original output. Never return the insufficient seed
        // or scan to that amount outside the declared work bound.
        let legs = [(1, 4001, 0, &[][..]), (3997, 6, 0, &[5000, 5000][..])];
        assert!((1..=64).all(|offer| integer_route(offer, &legs) == 0));
        assert_eq!(integer_route(1000, &legs), 1);
        s.references.prices.get_mut(&1).unwrap().price = "0.001".parse().unwrap();
        let mut work = WorkBudget {
            remaining: MAX_INVERSE_PROBES,
        };
        assert!(inverse_candidate(&s, &path, 1, SCALE / 2, &mut work).is_none());
        assert_eq!(
            work.remaining,
            MAX_INVERSE_PROBES - 2 * MAX_FORWARD_REFINEMENTS
        );
        let mut alternate = s.mirrors["pair7b"].clone();
        alternate.asset_0_addr = "in18".into();
        alternate.reserve_0 = 2000;
        alternate.reserve_1 = 3;
        s.mirrors.insert("alternate".into(), alternate);
        let healthy = vec![("alternate".into(), 1, 2)];
        let (required, quote) = select_inverse(&s, &[path, healthy.clone()], 1, SCALE / 2).unwrap();
        assert_eq!(required, 1000);
        assert_eq!(quote.path, healthy);
        assert_eq!(quote.output, 1);
    }

    #[test]
    fn inverse_pool_integer_closed_form_has_first_reaching_fee_and_reserve_bounds() {
        for fee in [0, 1, 333, 5000, 9999] {
            for rin in [1, 17, 1000] {
                let rout = 1000;
                for target in [1, 2, 17, 100, 999, 1000] {
                    if let Some(required) = inverse_pool_offer(rin, rout, fee, target) {
                        let output = |offer| {
                            let gross = rout * offer / (rin + offer);
                            gross - gross * u128::from(fee) / 10_000
                        };
                        assert!(output(required) >= target);
                        assert!(output(required - 1) < target);
                    } else {
                        assert!(rout - 1 - (rout - 1) * u128::from(fee) / 10_000 < target);
                    }
                }
            }
        }
        assert!(inverse_pool_offer(0, 1000, 0, 1).is_none());
        assert!(inverse_pool_offer(1, 1000, 10_000, 1).is_none());
        assert!(inverse_pool_offer(u128::MAX, u128::MAX, 0, u128::MAX - 1).is_none());
    }
}
