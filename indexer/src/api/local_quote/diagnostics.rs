//! Bounded internal evidence for candidate loss; request evaluation reads only
//! local state. Labels and logs never include addresses, actors or endpoints.
use super::pair_admission::AdmissionFailure;
use super::*;

#[derive(Clone, Default)]
pub(super) struct Inventory {
    pub pairs: Vec<pairs::PairRow>,
    pub rejected: HashMap<String, AdmissionFailure>,
}
pub(super) fn inventory(namespace: &str, pairs: &[pairs::PairRow]) {
    let mut g = store().lock().unwrap();
    if g.admission_inventory.len() >= 32 && !g.admission_inventory.contains_key(namespace) {
        return;
    }
    let current = g
        .admission_inventory
        .entry(namespace.to_owned())
        .or_default();
    current.pairs = pairs.iter().take(MAX_PAIRS).cloned().collect();
    let retained = current
        .pairs
        .iter()
        .map(|p| p.contract_address.as_str())
        .collect::<HashSet<_>>();
    current
        .rejected
        .retain(|pair, _| retained.contains(pair.as_str()));
}
pub(super) fn record(namespace: &str, pair: &str, reason: Option<AdmissionFailure>) {
    let mut g = store().lock().unwrap();
    let Some(inventory) = g.admission_inventory.get_mut(namespace) else {
        return;
    };
    if let Some(reason) = reason {
        // Losing a provider is not authoritative evidence that a previously
        // observed unsupported/protected prerequisite became ordinary again.
        if reason == AdmissionFailure::Missing
            && inventory
                .rejected
                .get(pair)
                .is_some_and(|old| *old != AdmissionFailure::Missing)
        {
            return;
        }
        if inventory.rejected.len() < MAX_PAIRS || inventory.rejected.contains_key(pair) {
            inventory.rejected.insert(pair.to_owned(), reason);
        }
    } else {
        inventory.rejected.remove(pair);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reason {
    Missing,
    Stale,
    Unsupported,
    Protection,
}
impl Reason {
    fn label(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Stale => "stale",
            Self::Unsupported => "unsupported",
            Self::Protection => "protection",
        }
    }
}
fn prerequisites(
    s: &Snapshot,
    path: &[route_paths::PathHop],
    at: i64,
    plan: Option<&[Option<route_solver::HybridHopJson>]>,
) -> Option<Reason> {
    if !protocol_guards::permits_path(s, path) {
        return Some(Reason::Protection);
    }
    for (hop_index, (pair, offer, ask)) in path.iter().enumerate() {
        if s.config_expiries
            .get(pair)
            .copied()
            .unwrap_or(s.config_expiry_ms)
            <= at
        {
            return Some(Reason::Stale);
        }
        let needs_book = plan
            .and_then(|plan| plan.get(hop_index))
            .and_then(Option::as_ref)
            .is_some_and(|hop| {
                hop.book_input
                    .parse::<u128>()
                    .is_ok_and(|amount| amount > 0)
            });
        let pool = (!needs_book)
            .then(|| s.pool_mirrors.get(pair))
            .flatten()
            .filter(|mirror| {
                !sim::is_snapshot_stale(mirror.snapshot_at, observation_max_age(s))
                    && matches!(
                        mirror.freshness,
                        sim::MirrorFreshness::Fresh | sim::MirrorFreshness::EmptyPool
                    )
                    && crate::indexer::fee_phase::permits_view(
                        &s.namespace,
                        mirror.pair_id,
                        mirror.block_height,
                        mirror.snapshot_at,
                        true,
                    )
            });
        let Some(mirror) = pool.or_else(|| s.mirrors.get(pair)) else {
            return Some(Reason::Missing);
        };
        if !crate::indexer::fee_phase::permits_view(
            &s.namespace,
            mirror.pair_id,
            mirror.block_height,
            mirror.snapshot_at,
            pool.is_some(),
        ) {
            return Some(Reason::Missing);
        }
        if sim::is_snapshot_stale(mirror.snapshot_at, observation_max_age(s))
            || mirror.freshness == sim::MirrorFreshness::Stale
        {
            return Some(Reason::Stale);
        }
        if mirror.freshness == sim::MirrorFreshness::MissingReserves {
            return Some(Reason::Missing);
        }
        for id in [offer, ask] {
            let Some(token) = s.addresses.get(id) else {
                return Some(Reason::Missing);
            };
            if let Some(tax) = s.taxes.get(token) {
                if tax.observed_at_ms > at || at.saturating_sub(tax.observed_at_ms) >= CONFIG_TTL_MS
                {
                    return Some(Reason::Stale);
                }
                if tax.rates(s.tax_trader.as_deref(), pair).is_none() {
                    return Some(match tax_rejection(tax, s.tax_trader.as_deref(), pair) {
                        Rejection::Stale => Reason::Stale,
                        Rejection::TradingBlocked => Reason::Protection,
                        _ => Reason::Missing,
                    });
                }
            }
        }
    }
    if let (Some(first), Some(last)) = (path.first(), path.last()) {
        for id in [first.1, last.2] {
            let Some(price) = s.references.prices.get(&id) else {
                return Some(Reason::Missing);
            };
            if price.decimals > 38 || price.price <= bigdecimal::BigDecimal::from(0) {
                return Some(Reason::Unsupported);
            }
            let observed = price.as_of.timestamp_millis();
            if observed > at || at.saturating_sub(observed) > 20_000 {
                return Some(Reason::Stale);
            }
        }
    }
    None
}
#[cfg(test)]
fn classify(
    s: &Snapshot,
    paths: &[Vec<route_paths::PathHop>],
    inventory: Option<&Inventory>,
    start: i32,
    goal: i32,
) -> Reason {
    classify_with_plan(s, paths, inventory, start, goal, None)
}
fn classify_with_plan(
    s: &Snapshot,
    paths: &[Vec<route_paths::PathHop>],
    inventory: Option<&Inventory>,
    start: i32,
    goal: i32,
    plan: Option<&[Option<route_solver::HybridHopJson>]>,
) -> Reason {
    // A complete fresh candidate can genuinely fail the requested budget/floor.
    // An unrelated broken alternative never changes that protection rejection.
    if paths
        .iter()
        .filter(|path| plan.is_none_or(|plan| plan.len() == path.len()))
        .any(|p| prerequisites(s, p, now_ms(), plan).is_none())
    {
        return Reason::Protection;
    }
    let failures = paths
        .iter()
        .filter(|path| plan.is_none_or(|plan| plan.len() == path.len()))
        .filter_map(|path| prerequisites(s, path, now_ms(), plan))
        .collect::<Vec<_>>();
    // A protected corridor must not hide an otherwise recoverable alternative.
    for reason in [Reason::Missing, Reason::Stale, Reason::Unsupported] {
        if failures.contains(&reason) {
            return reason;
        }
    }
    if !failures.is_empty() {
        return Reason::Protection;
    }
    let Some(inventory) = inventory else {
        return if s.pairs.is_empty() {
            Reason::Missing
        } else {
            Reason::Protection
        };
    };
    let catalog_paths = route_paths::find_paths_top_k_with_work_budget(
        start,
        goal,
        &inventory.pairs,
        4,
        1024,
        MAX_WORK,
    )
    .0;
    let mut unsupported = false;
    for path in catalog_paths {
        let failures = path
            .iter()
            .filter_map(|(pair, _, _)| {
                if s.pairs.iter().any(|p| &p.contract_address == pair) {
                    None
                } else {
                    Some(
                        inventory
                            .rejected
                            .get(pair)
                            .copied()
                            .unwrap_or(AdmissionFailure::Missing),
                    )
                }
            })
            .collect::<Vec<_>>();
        if failures.iter().any(|r| *r == AdmissionFailure::Missing) {
            return Reason::Missing;
        }
        unsupported |= failures.iter().any(|r| *r == AdmissionFailure::Unsupported);
    }
    if unsupported {
        Reason::Unsupported
    } else {
        Reason::Protection
    }
}
pub(super) fn candidate_error(
    namespace: &str,
    s: &Snapshot,
    paths: &[Vec<route_paths::PathHop>],
    start: i32,
    goal: i32,
    plan: Option<&[Option<route_solver::HybridHopJson>]>,
) -> Error {
    let inventory = store()
        .lock()
        .unwrap()
        .admission_inventory
        .get(namespace)
        .cloned();
    let reason = classify_with_plan(s, paths, inventory.as_ref(), start, goal, plan);
    tracing::debug!(
        reason = reason.label(),
        candidates = paths.len(),
        admitted_pairs = s.pairs.len(),
        mirrors = s.mirrors.len(),
        "Local quote candidate unavailable"
    );
    let error = match reason {
        Reason::Protection => {
            if paths.is_empty()
                && inventory.as_ref().is_none_or(|i| {
                    route_paths::find_paths_top_k_with_work_budget(
                        start, goal, &i.pairs, 4, 1024, MAX_WORK,
                    )
                    .0
                    .is_empty()
                })
            {
                Rejection::NoRoute.error()
            } else if paths
                .iter()
                .any(|p| prerequisites(s, p, now_ms(), plan).is_none())
            {
                Rejection::Protection.error()
            } else {
                Rejection::TradingBlocked.error()
            }
        }
        Reason::Unsupported => Rejection::Unsupported.error(),
        Reason::Missing => Rejection::Missing.error(),
        Reason::Stale => Rejection::Stale.error(),
    };
    bound_error(s, error)
}

/// Fixed-cardinality public contract. Never carry assets, actors or upstream bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Rejection {
    Missing,
    Stale,
    Unsupported,
    Dust,
    Liquidity,
    Accounting,
    Protection,
    TradingBlocked,
    NoRoute,
    InvalidRequest,
    AmountLimit,
    FairDeviation,
}
impl Rejection {
    pub(super) fn code(self) -> &'static str {
        match self {
            Self::Missing => "missing_observations",
            Self::Stale => "stale_observations",
            Self::Unsupported => "unsupported_semantics",
            Self::Dust => "amount_too_small",
            Self::Liquidity => "insufficient_liquidity",
            Self::Accounting => "accounting_unavailable",
            Self::Protection => "protection",
            Self::TradingBlocked => "trading_blocked",
            Self::NoRoute => "no_route",
            Self::InvalidRequest => "invalid_request",
            Self::AmountLimit => "amount_too_large",
            Self::FairDeviation => "price_impact_too_high",
        }
    }
    pub(super) fn priority(self) -> u8 {
        // Real fully priced protection wins over broken alternatives. Never let
        // a missing alternative turn a confirmed dust/liquidity rejection generic.
        match self {
            Self::Protection => 0,
            Self::FairDeviation => 1,
            Self::Dust => 2,
            Self::Liquidity | Self::AmountLimit => 3,
            Self::Accounting => 4,
            Self::Missing => 5,
            Self::Stale => 6,
            Self::Unsupported => 7,
            Self::TradingBlocked => 8,
            Self::NoRoute => 9,
            Self::InvalidRequest => 10,
        }
    }
    pub(super) fn error(self) -> Error {
        let status = match self {
            Self::InvalidRequest => StatusCode::BAD_REQUEST,
            Self::Missing | Self::Stale | Self::Accounting => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        };
        (status, json!({"error":self.code()}).to_string())
    }
}
pub(super) fn path_prerequisite_rejection(
    s: &Snapshot,
    path: &[route_paths::PathHop],
) -> Option<Rejection> {
    prerequisites(s, path, now_ms(), None).map(|reason| match reason {
        Reason::Missing => Rejection::Missing,
        Reason::Stale => Rejection::Stale,
        Reason::Unsupported => Rejection::Unsupported,
        Reason::Protection => Rejection::TradingBlocked,
    })
}
pub(super) fn bound_error(s: &Snapshot, mut error: Error) -> Error {
    if let Ok(mut body) = serde_json::from_str::<Value>(&error.1) {
        if body["generation"].is_null() {
            body["generation"] = json!(s.generation);
        }
        error.1 = body.to_string();
    }
    error
}
pub(super) fn tax_rejection(
    tax: &tax::TokenObservation,
    actor: Option<&str>,
    pair: &str,
) -> Rejection {
    let at = now_ms();
    if actor
        .and_then(|a| tax.actors.get(a))
        .is_some_and(|a| a.protocol)
    {
        return Rejection::TradingBlocked;
    }
    if tax.observed_at_ms > at || at.saturating_sub(tax.observed_at_ms) >= CONFIG_TTL_MS {
        return Rejection::Stale;
    }
    if !tax.pairs.contains(pair) {
        return Rejection::Missing;
    }
    if let Some(actor) = actor {
        let Some(observed) = tax.actors.get(actor) else {
            return Rejection::Missing;
        };
        if observed.observed_at_ms > at
            || at.saturating_sub(observed.observed_at_ms) >= CONFIG_TTL_MS
        {
            return Rejection::Stale;
        }
        if tax.max_wallet.is_some() && observed.balance.is_none() {
            return Rejection::Missing;
        }
    } else if tax.max_wallet.is_some() {
        return Rejection::Missing;
    }
    Rejection::TradingBlocked
}
pub(super) fn simulation_rejection(error: sim::DbSimError) -> Rejection {
    match error {
        sim::DbSimError::InsufficientLiquidity | sim::DbSimError::ImplausibleHop => {
            Rejection::Liquidity
        }
        sim::DbSimError::StaleMirror => Rejection::Stale,
        sim::DbSimError::MissingMirror
        | sim::DbSimError::IncompleteFeeGeneration { .. }
        | sim::DbSimError::FeeGenerationRejected => Rejection::Missing,
        _ => Rejection::Accounting,
    }
}
pub(super) fn mirror_rejection(s: &Snapshot, pair: &str, needs_book: bool) -> Rejection {
    let views = s
        .mirrors
        .get(pair)
        .into_iter()
        .chain((!needs_book).then(|| s.pool_mirrors.get(pair)).flatten());
    if views.clone().any(|m| {
        sim::is_snapshot_stale(m.snapshot_at, observation_max_age(s))
            || m.freshness == sim::MirrorFreshness::Stale
    }) {
        Rejection::Stale
    } else {
        Rejection::Missing
    }
}
pub(super) fn reference_rejection(
    s: &Snapshot,
    path: &[route_paths::PathHop],
    input: u128,
) -> Rejection {
    for id in [path[0].1, path.last().unwrap().2] {
        let Some(p) = s.references.prices.get(&id) else {
            return Rejection::Missing;
        };
        if p.decimals > 38 || p.price <= bigdecimal::BigDecimal::from(0) {
            return Rejection::Unsupported;
        }
        if p.as_of > Utc::now() || now_ms().saturating_sub(p.as_of.timestamp_millis()) > 20_000 {
            return Rejection::Stale;
        }
    }
    // Both fresh marks exist, but integer conversion is unrepresentable. A
    // zero reference is not zero impact and must never admit execution.
    let a = &s.references.prices[&path[0].1];
    let b = &s.references.prices[&path.last().unwrap().2];
    let unrounded = bigdecimal::BigDecimal::from(input)
        * &a.price
        * bigdecimal::BigDecimal::from(10u128.pow(b.decimals))
        / (&b.price * bigdecimal::BigDecimal::from(10u128.pow(a.decimals)));
    if unrounded < bigdecimal::BigDecimal::from(1) {
        Rejection::Dust
    } else {
        Rejection::Accounting
    }
}
pub(super) fn selection_error(
    namespace: &str,
    s: &Snapshot,
    paths: &[Vec<route_paths::PathHop>],
    start: i32,
    goal: i32,
    plan: Option<&[Option<route_solver::HybridHopJson>]>,
    q: &route_solver::SolveRouteParams,
    input: u128,
    spread: u128,
    native: Option<&NativeContext>,
    selected: Selection,
) -> Error {
    let reason = selected.rejected;
    let indicative = selected.indicative;
    let Some(reason) = reason else {
        return candidate_error(namespace, s, paths, start, goal, plan);
    };
    tracing::debug!(
        reason = reason.code(),
        candidates = paths.len(),
        "Local quote candidate rejected"
    );
    let mut error = reason.error();
    if reason == Rejection::Protection {
        if let Some(body) =
            indicative.and_then(|c| final_response(s, q, &c, input, spread, paths.len(), native))
        {
            let expiry = body["execution_metadata"]["expires_at_ms"]
                .as_i64()
                .unwrap_or(0);
            if expiry > now_ms() {
                error.1 = json!({"error":reason.code(),"indicative_amount_out":body["estimated_amount_out_net"],
                    "execution_supported":false,"generation":body["execution_metadata"]["generation"],
                    "observed_at_ms":body["execution_metadata"]["observed_at_ms"],"expires_at_ms":expiry}).to_string();
            }
        }
    }
    bound_error(s, error)
}
#[cfg(test)]
pub(super) fn amount_error(
    namespace: &str,
    s: &Snapshot,
    paths: &[Vec<route_paths::PathHop>],
    start: i32,
    goal: i32,
    plan: Option<&[Option<route_solver::HybridHopJson>]>,
    q: &route_solver::SolveRouteParams,
    input: u128,
    spread: u128,
    book_allowed: bool,
    maker_fills: u32,
    native: Option<&NativeContext>,
) -> Error {
    selection_error(
        namespace,
        s,
        paths,
        start,
        goal,
        plan,
        q,
        input,
        spread,
        native,
        select_detailed(s, paths, input, spread, book_allowed, maker_fills, plan),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn issue1473_known_tax_policy_requires_original_actor_data_and_preserves_negatives() {
        let mut s = super::super::tests::fixture();
        s.tax_trader = Some("synthetic-wallet".into());
        s.taxes.insert(
            "out18".into(),
            tax::TokenObservation {
                policy_identity: "synthetic".into(),
                code_id: 1,
                buy_bps: 0,
                sell_bps: 0,
                observed_at_ms: now_ms(),
                max_wallet: Some(100),
                pairs: HashSet::from(["pair7b".into()]),
                actors: Default::default(),
            },
        );
        let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
        let classify = |s: &Snapshot| {
            evaluate_with_plan(s, &path, 10u128.pow(18), SCALE / 20, 0, 8, None).err()
        };
        assert_eq!(classify(&s), Some(Rejection::Missing));
        s.taxes.get_mut("out18").unwrap().actors.insert(
            "synthetic-wallet".into(),
            tax::ActorObservation {
                observed_at_ms: now_ms(),
                protocol: false,
                manager: false,
                balance: None,
            },
        );
        assert_eq!(
            classify(&s),
            Some(Rejection::Missing),
            "missing balance cannot become a known wallet-limit violation"
        );
        s.taxes
            .get_mut("out18")
            .unwrap()
            .actors
            .get_mut("synthetic-wallet")
            .unwrap()
            .balance = Some(0);
        assert_eq!(
            classify(&s),
            Some(Rejection::TradingBlocked),
            "fresh known credit exceeds its cap"
        );
        s.taxes.get_mut("out18").unwrap().max_wallet = Some(2 * 10u128.pow(18));
        assert!(classify(&s).is_none());
        s.taxes
            .get_mut("out18")
            .unwrap()
            .actors
            .get_mut("synthetic-wallet")
            .unwrap()
            .observed_at_ms -= CONFIG_TTL_MS;
        assert_eq!(classify(&s), Some(Rejection::Stale));
        s.taxes
            .get_mut("out18")
            .unwrap()
            .actors
            .get_mut("synthetic-wallet")
            .unwrap()
            .protocol = true;
        assert_eq!(
            classify(&s),
            Some(Rejection::TradingBlocked),
            "known protocol negatives cannot be erased by age"
        );
    }
    #[test]
    fn missing_stale_prerequisites_are_not_amount_protection_and_healthy_alternative_wins() {
        let mut s = super::super::tests::fixture();
        let p = vec![("pair0a".into(), 1, 3), ("pair0b".into(), 3, 2)];
        assert_eq!(classify(&s, &[p.clone()], None, 1, 2), Reason::Protection);
        s.mirrors.remove("pair0a");
        assert_eq!(classify(&s, &[p.clone()], None, 1, 2), Reason::Missing);
        s.config_expiries.insert("pair0a".into(), now_ms() - 1);
        assert_eq!(classify(&s, &[p.clone()], None, 1, 2), Reason::Stale);
        let healthy = vec![("pair1a".into(), 1, 4), ("pair1b".into(), 4, 2)];
        assert_eq!(classify(&s, &[p, healthy], None, 1, 2), Reason::Protection);
    }
    #[test]
    fn catalog_corridor_missing_configuration_is_unavailable_without_masking_known_guard() {
        let mut s = super::super::tests::fixture();
        let mut inventory = Inventory {
            pairs: s.pairs.clone(),
            rejected: HashMap::new(),
        };
        s.pairs.clear();
        assert_eq!(classify(&s, &[], Some(&inventory), 1, 2), Reason::Missing);
        inventory.rejected.extend(
            inventory
                .pairs
                .iter()
                .map(|p| (p.contract_address.clone(), AdmissionFailure::Protected)),
        );
        assert_eq!(
            classify(&s, &[], Some(&inventory), 1, 2),
            Reason::Protection
        );
        inventory
            .rejected
            .insert("pair0a".into(), AdmissionFailure::Unsupported);
        assert_eq!(
            classify(&s, &[], Some(&inventory), 1, 2),
            Reason::Unsupported
        );
    }
    #[test]
    fn recovery1464_declared_book_expiry_is_unavailable_even_with_fresh_pool() {
        let mut s = super::super::tests::fixture();
        let path = vec![("pair0a".into(), 1, 3), ("pair0b".into(), 3, 2)];
        let pool = s.mirrors["pair0a"].clone();
        s.pool_mirrors.insert("pair0a".into(), pool);
        Arc::make_mut(s.mirrors.get_mut("pair0a").unwrap()).snapshot_at =
            Utc::now() - chrono::Duration::seconds(21);
        let plan = [
            Some(route_solver::HybridHopJson {
                pool_input: "0".into(),
                book_input: "1".into(),
                max_maker_fills: 8,
                book_start_hint: None,
            }),
            None,
        ];
        assert!(classify(&s, std::slice::from_ref(&path), None, 1, 2) == Reason::Protection);
        assert!(classify_with_plan(&s, &[path], None, 1, 2, Some(&plan)) == Reason::Stale);
    }

    #[test]
    fn recovery1464_unusable_pool_does_not_hide_complete_hybrid_protection() {
        let mut s = super::super::tests::fixture();
        let mut pool = (*s.mirrors["pair0a"]).clone();
        pool.freshness = sim::MirrorFreshness::MissingReserves;
        s.pool_mirrors.insert("pair0a".into(), Arc::new(pool));
        let path = vec![("pair0a".into(), 1, 3), ("pair0b".into(), 3, 2)];
        assert!(super::super::quote_mirror(&s, "pair0a", false).is_some());
        assert!(classify(&s, &[path], None, 1, 2) == Reason::Protection);
    }

    #[test]
    #[serial_test::serial]
    fn recovery1464_regressed_pool_does_not_hide_eligible_original_hybrid() {
        let mut s = super::super::tests::fixture();
        s.namespace = "diagnostic-pool-regression".into();
        let hybrid = s.mirrors["pair0a"].clone();
        s.pool_mirrors.insert("pair0a".into(), hybrid.clone());
        let mut newer = (*hybrid).clone();
        newer.block_height = Some(11);
        newer.snapshot_at += chrono::Duration::microseconds(1);
        assert!(crate::indexer::fee_phase::qualify(
            &s.namespace,
            vec![crate::indexer::fee_phase::Qualification {
                pair_id: hybrid.pair_id,
                assets: (1, 3),
                context: "diagnostic-regression".into(),
                expiry_ms: now_ms() + 10_000,
                max_age_ms: s.max_age_ms,
                mirror: Some(hybrid.clone()),
                pool_mirror: Some(Arc::new(newer)),
            }]
        ));
        assert!(Arc::ptr_eq(
            super::super::quote_mirror(&s, "pair0a", false).unwrap(),
            &hybrid
        ));
        let path = vec![("pair0a".into(), 1, 3), ("pair0b".into(), 3, 2)];
        assert!(classify(&s, &[path], None, 1, 2) == Reason::Protection);
        assert!(crate::indexer::fee_phase::qualify(&s.namespace, Vec::new()));
    }
}
