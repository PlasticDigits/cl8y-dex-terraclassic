//! Canonical identity of the effective admitted market, independent of publication history.
//! Producer epochs and extended configuration deadlines do not renew reviewed work;
//! admission_generation separately binds monotone validity, fee/freeze/protection guards.
use super::*;

fn base(s: &Snapshot) -> Sha256 {
    let mut digest = Sha256::new();
    digest.update(format!("effective-market-v2|{}", s.namespace));
    let mut addresses = s.addresses.iter().collect::<Vec<_>>();
    addresses.sort_by_key(|(id, _)| **id);
    digest.update(format!("{addresses:?}"));
    let mut contexts = s.pair_contexts.iter().collect::<Vec<_>>();
    contexts.sort_by_key(|(address, _)| *address);
    digest.update(format!("{contexts:?}"));
    let mut hook_keys = s.hooks.iter().collect::<Vec<_>>();
    hook_keys.sort_by_key(|(k, _)| *k);
    for (address, prices) in hook_keys {
        digest.update(format!("{address}|{:?}", prices.observation_identity()));
    }
    let mut refs = s.references.prices.iter().collect::<Vec<_>>();
    refs.sort_by_key(|(id, _)| **id);
    digest.update(format!("{refs:?}"));
    let mut exclusions = s.references.policy_exclusions.iter().collect::<Vec<_>>();
    exclusions.sort_by_key(|(id, _)| **id);
    digest.update(format!("{}|{exclusions:?}", s.references.policy_identity));
    let mut discounted = s.discounted_pairs.iter().collect::<Vec<_>>();
    discounted.sort();
    digest.update(format!("{discounted:?}"));
    let mut tax_fields = s.taxes.iter().collect::<Vec<_>>();
    tax_fields.sort_by_key(|(address, _)| *address);
    for (address, token) in tax_fields {
        let mut registered = token.pairs.iter().collect::<Vec<_>>();
        registered.sort();
        digest.update(format!(
            "{address}|{}|{}|{}|{}|{:?}|{registered:?}|{:?}|{}",
            token.code_id,
            token.buy_bps,
            token.sell_bps,
            token.observed_at_ms,
            token.max_wallet,
            token.actors,
            token.policy_identity
        ));
    }
    digest.update(&s.policy_identity);
    digest
}

#[derive(Clone)]
pub(super) struct ViewHash {
    mirror: std::sync::Weak<sim::HopMirror>,
    digest: [u8; 32],
    address: String,
    pool: bool,
}
pub(super) type ViewHashes = HashMap<(String, bool), ViewHash>;

fn prepare_views(s: &Snapshot) -> ViewHashes {
    let mut hashes = HashMap::new();
    for (pool, views) in [(false, &s.mirrors), (true, &s.pool_mirrors)] {
        for (address, mirror) in views {
            hashes.insert(
                (address.clone(), pool),
                ViewHash {
                    mirror: Arc::downgrade(mirror),
                    address: address.clone(),
                    pool,
                    digest: hash_view(if pool { "pool" } else { "hybrid" }, address, mirror),
                },
            );
        }
    }
    hashes
}

/// Full immutable book/context hashing happens before taking Store, as before.
pub(super) fn prepare(s: &mut Snapshot) -> Sha256 {
    s.view_hashes = Arc::new(prepare_views(s));
    base(s)
}

/// Preservation substitutes already-bound digests; this composition is bounded
/// by admitted view count, rather than the total number of resting orders.
pub(super) fn finish(s: &Snapshot, mut digest: Sha256) -> Option<String> {
    let mut actor_fields = s.actors.iter().collect::<Vec<_>>();
    actor_fields.sort_by_key(|(address, _)| *address);
    digest.update(format!("{actor_fields:?}"));
    digest.update(format!("actor-revision:{}", s.actor_revision));
    // Admission changes invalidate quotes even if reserve amounts did not change.
    let mut admitted: Vec<_> = s
        .pairs
        .iter()
        .map(|p| (&p.contract_address, p.asset_0_id, p.asset_1_id))
        .collect();
    admitted.sort();
    digest.update(format!("{admitted:?}"));

    for (pool, views) in [(false, &s.mirrors), (true, &s.pool_mirrors)] {
        let mut ordered = views.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|(address, _)| *address);
        for (address, mirror) in ordered {
            let hash = s.view_hashes.get(&(address.clone(), pool))?;
            if hash.address != *address || hash.pool != pool {
                return None;
            }
            let bound = hash.mirror.upgrade()?;
            if !Arc::ptr_eq(&bound, mirror) {
                return None;
            }
            digest.update(format!("{pool}|{address}|"));
            digest.update(hash.digest);
        }
    }
    Some(format!("{:x}", digest.finalize()))
}

#[cfg(test)]
pub(super) fn effective_market(s: &Snapshot) -> String {
    let mut prepared = s.clone();
    let base = prepare(&mut prepared);
    finish(&prepared, base).expect("unmodified prepared views are bound")
}

fn hash_view(kind: &str, address: &str, m: &Arc<sim::HopMirror>) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(format!(
        "{kind}|{address}|{}|{:?}|{}|{}|{}|{:?}|{}|{}|{}",
        m.pair_id,
        m.freshness,
        m.asset_0_addr,
        m.asset_1_addr,
        m.fee_bps,
        m.block_height,
        m.snapshot_at,
        m.reserve_0,
        m.reserve_1
    ));
    // Vector position preserves DLL/equal-price FIFO; sides and owner context
    // cannot alias even if numeric order fields happen to be identical.
    for (side, orders) in [("bids", &m.bids), ("asks", &m.asks)] {
        digest.update(side);
        for order in orders {
            digest.update(format!(
                "{}|{}|{}|{}|{}|{:?}|{:?}",
                order.pair_id,
                order.order_id,
                order.side,
                order.price,
                order.remaining,
                order.expires_at,
                order.owner
            ));
        }
    }
    digest.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn issue1489_canonical_market_hash_binds_effective_fields_and_preserves_book_fifo() {
        let source = super::super::tests::fixture();
        let baseline = effective_market(&source);
        let mut permuted = source.clone();
        permuted.pairs.reverse();
        permuted.addresses = source
            .addresses
            .iter()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        permuted.mirrors = source
            .mirrors
            .iter()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        permuted.references.prices = source
            .references
            .prices
            .iter()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        assert_eq!(effective_market(&permuted), baseline);
        // Opaque history and routine epochs are excluded; explicit admission
        // guards bind them where needed, and original job deadlines stay pinned.
        permuted.generation = "different-history".into();
        permuted.config_epoch += 1;
        permuted.config_expiry_ms += 1000;
        assert_eq!(effective_market(&permuted), baseline);
        let changed: [(&str, fn(&mut Snapshot)); 15] = [
            ("namespace", |s| s.namespace.push_str("-other")),
            ("address", |s| {
                s.addresses.insert(1, "different".into());
            }),
            ("pair membership", |s| {
                s.pairs.pop();
            }),
            ("pair binding", |s| s.pairs[0].asset_0_id += 100),
            ("qualification", |s| {
                s.pair_contexts.insert("pair0a".into(), "changed".into());
            }),
            ("fee", |s| {
                Arc::make_mut(s.mirrors.get_mut("pair0a").unwrap()).fee_bps += 1
            }),
            ("height", |s| {
                Arc::make_mut(s.mirrors.get_mut("pair0a").unwrap()).block_height = Some(11)
            }),
            ("timestamp", |s| {
                Arc::make_mut(s.mirrors.get_mut("pair0a").unwrap()).snapshot_at +=
                    chrono::Duration::milliseconds(1)
            }),
            ("reserve", |s| {
                Arc::make_mut(s.mirrors.get_mut("pair0a").unwrap()).reserve_0 += 1
            }),
            ("freshness", |s| {
                Arc::make_mut(s.mirrors.get_mut("pair0a").unwrap()).freshness =
                    sim::MirrorFreshness::Stale
            }),
            ("pool companion", |s| {
                s.pool_mirrors
                    .insert("pair0a".into(), s.mirrors["pair0a"].clone());
            }),
            ("blocked actor", |s| {
                s.actors.insert("actor".into(), (0, true));
            }),
            ("discount", |s| {
                s.discounted_pairs.insert("pair0a".into());
            }),
            ("reference", |s| {
                s.references.prices.get_mut(&1).unwrap().decimals += 1
            }),
            ("policy", |s| s.policy_identity.push_str("-changed")),
        ];
        for (name, change) in changed {
            let mut s = source.clone();
            change(&mut s);
            assert_ne!(effective_market(&s), baseline, "{name} must invalidate");
        }
        let mut contextual = source.clone();
        contextual.taxes.insert(
            "tax-token".into(),
            tax::TokenObservation {
                policy_identity: "tax-policy".into(),
                code_id: 10,
                buy_bps: 100,
                sell_bps: 200,
                observed_at_ms: now_ms(),
                max_wallet: Some(1000),
                pairs: HashSet::from(["pair0a".into(), "pair0b".into()]),
                actors: std::collections::BTreeMap::from([(
                    "actor".into(),
                    tax::ActorObservation {
                        observed_at_ms: now_ms(),
                        protocol: false,
                        manager: false,
                        balance: Some(100),
                    },
                )]),
            },
        );
        contextual.hooks.insert(
            "pair0a".into(),
            super::super::super::local_hook_pricing::CachedHookPrices::from_observation(&[(
                "hook".into(),
                json!({"recipient":"recipient","tax_percentage_bps":50,"tax_token":"in18"}),
            )])
            .unwrap(),
        );
        let contextual_hash = effective_market(&contextual);
        let contextual_changes: [(&str, fn(&mut Snapshot)); 9] = [
            ("tax fee", |s| {
                s.taxes.get_mut("tax-token").unwrap().buy_bps += 1
            }),
            ("tax qualification time", |s| {
                s.taxes.get_mut("tax-token").unwrap().observed_at_ms += 1
            }),
            ("tax max wallet", |s| {
                s.taxes.get_mut("tax-token").unwrap().max_wallet = Some(2000)
            }),
            ("tax registration", |s| {
                s.taxes.get_mut("tax-token").unwrap().pairs.remove("pair0a");
            }),
            ("tax protocol actor", |s| {
                s.taxes
                    .get_mut("tax-token")
                    .unwrap()
                    .actors
                    .get_mut("actor")
                    .unwrap()
                    .protocol = true
            }),
            ("tax balance", |s| {
                s.taxes
                    .get_mut("tax-token")
                    .unwrap()
                    .actors
                    .get_mut("actor")
                    .unwrap()
                    .balance = None
            }),
            ("hook", |s| {
                s.hooks.clear();
            }),
            ("reference policy", |s| {
                s.references.policy_identity.push_str("-changed")
            }),
            ("reference quality", |s| {
                s.references.policy_exclusions.insert(999, Utc::now());
            }),
        ];
        for (name, change) in contextual_changes {
            let mut s = contextual.clone();
            change(&mut s);
            assert_ne!(
                effective_market(&s),
                contextual_hash,
                "{name} must invalidate"
            );
        }
        let mut book = source.clone();
        let m = Arc::make_mut(book.mirrors.get_mut("pair0a").unwrap());
        for id in [1, 2] {
            m.bids
                .push(crate::db::queries::resting_orders::RestingOrderRow {
                    pair_id: m.pair_id,
                    order_id: id,
                    side: "bid".into(),
                    price: 1.into(),
                    remaining: 10.into(),
                    owner: None,
                    expires_at: None,
                });
        }
        let ordered = effective_market(&book);
        assert_ne!(ordered, baseline);
        Arc::make_mut(book.mirrors.get_mut("pair0a").unwrap())
            .bids
            .reverse();
        assert_ne!(
            effective_market(&book),
            ordered,
            "equal-price FIFO position is economic state"
        );
    }
    #[test]
    fn issue1489_prepared_identity_rejects_detached_mutated_or_wrong_view_bindings() {
        let mut original = super::super::tests::fixture();
        let base = prepare(&mut original);
        let identity = finish(&original, base.clone()).unwrap();
        let mut mutated = original.clone();
        Arc::make_mut(mutated.mirrors.get_mut("pair0a").unwrap()).reserve_0 += 1;
        assert!(
            finish(&mutated, base.clone()).is_none(),
            "changed immutable view must be rehashed outside Store"
        );
        let next = prepare(&mut mutated);
        assert_ne!(finish(&mutated, next).unwrap(), identity);
        let mut detached = original.clone();
        detached.mirrors.insert(
            "pair0a".into(),
            Arc::new((*original.mirrors["pair0a"]).clone()),
        );
        assert!(finish(&detached, base.clone()).is_none());
        let next = prepare(&mut detached);
        assert_eq!(
            finish(&detached, next).unwrap(),
            identity,
            "equal contents never hash allocation history"
        );
        for wrong_pool in [false, true] {
            let mut wrong = original.clone();
            let hash = Arc::make_mut(&mut wrong.view_hashes)
                .get_mut(&("pair0a".into(), false))
                .unwrap();
            if wrong_pool {
                hash.pool = true;
            } else {
                hash.address = "other-address".into();
            }
            assert!(
                finish(&wrong, base.clone()).is_none(),
                "view kind/address cannot borrow another binding"
            );
        }
        let mut missing = original.clone();
        Arc::make_mut(&mut missing.view_hashes).clear();
        assert!(finish(&missing, base.clone()).is_none());
    }
    #[test]
    fn issue1489_preserved_expired_qualification_never_renews_or_executes() {
        let mut previous = super::super::tests::fixture();
        previous.namespace = "expired-preservation1489".into();
        previous
            .pair_contexts
            .insert("pair7a".into(), "qualified".into());
        let expired = now_ms() - 1;
        previous.config_expiries.insert("pair7a".into(), expired);
        prepare(&mut previous);
        let mut incoming = previous.clone();
        Arc::make_mut(incoming.mirrors.get_mut("pair7a").unwrap()).snapshot_at -=
            chrono::Duration::milliseconds(1);
        incoming
            .config_expiries
            .insert("pair7a".into(), now_ms() + 60_000);
        let prepared = prepare(&mut incoming);
        preserve_newer_mirrors(&mut incoming, &previous);
        assert!(Arc::ptr_eq(
            &incoming.mirrors["pair7a"],
            &previous.mirrors["pair7a"]
        ));
        assert_eq!(incoming.config_expiries["pair7a"], expired);
        assert!(
            finish(&incoming, prepared).is_some(),
            "bound identity is separate from execution eligibility"
        );
        assert!(
            score(
                &incoming,
                &[("pair7a".into(), 1, 10)],
                10u128.pow(18),
                SCALE / 20
            )
            .is_none(),
            "original expired qualification cannot execute"
        );
        let mut stale = previous.clone();
        Arc::make_mut(stale.mirrors.get_mut("pair7a").unwrap()).snapshot_at -=
            chrono::Duration::seconds(21);
        prepare(&mut stale);
        let mut older = stale.clone();
        Arc::make_mut(older.mirrors.get_mut("pair7a").unwrap()).snapshot_at -=
            chrono::Duration::seconds(1);
        prepare(&mut older);
        let before = older.mirrors["pair7a"].clone();
        preserve_newer_mirrors(&mut older, &stale);
        assert!(
            Arc::ptr_eq(&older.mirrors["pair7a"], &before),
            "stale previous tuple cannot be retained"
        );
    }
}
