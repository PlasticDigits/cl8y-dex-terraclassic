//! The maximum profile owns a child process so its 512 qualified markers use
//! the fixed registry budget without changing or resetting other unit fixtures.
use super::*;
use bech32::ToBase32;
pub(super) fn address(id: u32) -> String {
    let mut raw = [0u8; 32];
    raw[..4].copy_from_slice(&id.to_be_bytes());
    bech32::encode("terra", raw.to_base32(), bech32::Variant::Bech32).unwrap()
}
fn maximum() -> Snapshot {
    let original = tests::fixture();
    let mut s = original.clone();
    s.namespace = "scheduled-publication1489".into();
    s.addresses = original
        .addresses
        .keys()
        .map(|id| (*id, address(10000 + *id as u32)))
        .collect();
    s.references.prices.get_mut(&1).unwrap().decimals = 18;
    s.references.prices.get_mut(&10).unwrap().decimals = 6;
    s.pairs.clear();
    s.mirrors.clear();
    for id in 0..MAX_PAIRS {
        let mut pair = original.pairs[14].clone();
        pair.id = id as i32;
        pair.contract_address = address(20000 + id as u32);
        let mut mirror = (*original.mirrors["pair7a"]).clone();
        mirror.pair_id = id as i32;
        mirror.asset_0_addr = s.addresses[&pair.asset_0_id].clone();
        mirror.asset_1_addr = s.addresses[&pair.asset_1_id].clone();
        for order in 0..2048 {
            let row = crate::db::queries::resting_orders::RestingOrderRow {
                pair_id: id as i32,
                order_id: order,
                side: if order < 1024 { "bid" } else { "ask" }.into(),
                price: 1.into(),
                remaining: 10.into(),
                owner: None,
                expires_at: None,
            };
            if order < 1024 {
                mirror.bids.push(row);
            } else {
                mirror.asks.push(row);
            }
        }
        s.pair_contexts
            .insert(pair.contract_address.clone(), format!("profile-{id}"));
        s.config_expiries
            .insert(pair.contract_address.clone(), s.config_expiry_ms);
        s.mirrors
            .insert(pair.contract_address.clone(), Arc::new(mirror));
        s.pairs.push(pair);
    }
    s
}
pub(super) fn pending(s: Snapshot) -> publication::Pending {
    publication(
        s.namespace,
        s.pairs,
        s.addresses,
        s.mirrors,
        s.pool_mirrors,
        s.max_age_ms,
        s.hooks,
        s.config_expiry_ms,
        s.config_expiries,
        s.pair_contexts,
        s.references,
        s.actors,
        s.discounted_pairs,
        s.policy_identity,
        s.taxes,
        None,
    )
}
fn validated(body: &Value) {
    assert!(
        body["execution_metadata"]["execution_supported"].is_null()
            || body["execution_metadata"]["execution_supported"] == true
    );
    assert_eq!(body["execution_metadata"]["version"], 1);
    assert_eq!(body["quote_kind"], "indexer_pool_db");
    assert_eq!(body["economic_attribution"]["status"], "available");
    assert!(body["router_operations"].as_array().unwrap().len() > 0);
    let parse = |v: &Value| v.as_str().unwrap().parse::<BigInt>().unwrap();
    let fair = parse(&body["spot_amount_out"]);
    let net = parse(&body["estimated_amount_out_net"]);
    assert!(net > BigInt::from(0));
    assert!(parse(&body["execution_metadata"]["minimum_receive"]) > BigInt::from(0));
    let total = [
        "market_movement",
        "reference_divergence",
        "trading_cost",
        "wrapper_loss",
    ]
    .iter()
    .map(|name| parse(&body["economic_attribution"]["components"][name]))
    .sum::<BigInt>();
    assert_eq!(total, fair - net, "signed raw attribution must reconcile");
}
// The child isolates registry capacity. Share the existing publication-fixture
// serial group so its heavy parent fixtures do not compete with this profile.
#[test]
#[serial_test::serial]
fn issue1489_scheduled_maximum_publication_keeps_initialized_http_quote_clock() {
    const NAME: &str = "api::local_quote::publication_tests::issue1489_scheduled_maximum_publication_keeps_initialized_http_quote_clock";
    if std::env::var_os("DEX1489_PUBLICATION_PROFILE_CHILD").is_some() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(maximum_http());
        return;
    }
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", NAME, "--nocapture"])
        .env("DEX1489_PUBLICATION_PROFILE_CHILD", "1")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "isolated maximum HTTP profile failed: {} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    for line in String::from_utf8_lossy(&result.stdout)
        .lines()
        .filter(|line| line.starts_with("maximum_current_pairs="))
    {
        println!("{line}");
    }
}
async fn maximum_http() {
    let source = maximum();
    let namespace = source.namespace.clone();
    pending(source.clone()).scheduled().await.unwrap();
    let mut q = tests::request(10u128.pow(18));
    q.token_in = source.addresses[&1].clone();
    q.token_out = source.addresses[&source.pairs[0].asset_1_id].clone();
    q.pool_only = Some(true);
    q.protection_policy = Some("quote_relative_v1".into());
    let handler_namespace = namespace.clone();
    let app = axum::Router::new().route(
        "/api/v1/route/solve",
        axum::routing::get(
            move |axum::extract::Query(q): axum::extract::Query<route_solver::SolveRouteParams>| {
                let namespace = handler_namespace.clone();
                async move { solve_local(&namespace, &q).await }
            },
        ),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::new();
    let url = format!("http://{address}/api/v1/route/solve");
    let query = [
        ("token_in", q.token_in.as_str()),
        ("token_out", q.token_out.as_str()),
        ("amount_in", q.amount_in.as_deref().unwrap()),
        ("pool_only", "true"),
        ("max_spread", "0.05"),
        ("protection_policy", "quote_relative_v1"),
    ];
    let response = client.get(&url).query(&query).send().await.unwrap();
    let status = response.status();
    let original: Value = response.json().await.unwrap();
    assert!(
        status.is_success(),
        "warmup status={status} error={} fair={}",
        original["error"],
        original["spot_amount_out"]
    );
    validated(&original);
    let mut delayed = source.clone();
    let first_pair = source.pairs[0].contract_address.clone();
    let mirror = Arc::make_mut(delayed.mirrors.get_mut(&first_pair).unwrap());
    mirror.snapshot_at -= chrono::Duration::milliseconds(1);
    mirror.reserve_0 /= 2;
    for expiry in delayed.config_expiries.values_mut() {
        *expiry += 10_000;
    }
    let publisher = tokio::spawn(async move {
        pending(delayed).scheduled().await.unwrap();
    });
    let mut count = 0;
    let mut maximum_ms = 0;
    let publication_started = std::time::Instant::now();
    // The read-count cap bounds fixture traffic, not background preparation.
    // Give cold CI CPUs time to hash the million-order snapshot while keeping
    // the original 500ms useful-response deadline and 100-request cap intact.
    let read_interval = Duration::from_millis(100);
    while !publisher.is_finished() || count < 2 {
        let started = std::time::Instant::now();
        let response: Value = client
            .get(&url)
            .query(&query)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        validated(&response);
        let elapsed = started.elapsed();
        maximum_ms = maximum_ms.max(elapsed.as_millis());
        assert!(
            elapsed <= Duration::from_millis(500),
            "initialized useful HTTP quote exceeded unchanged 500ms budget: {elapsed:?}"
        );
        assert_eq!(
            response["execution_metadata"]["generation"],
            original["execution_metadata"]["generation"]
        );
        assert_eq!(
            response["execution_metadata"]["expires_at_ms"],
            original["execution_metadata"]["expires_at_ms"]
        );
        assert!(
            response["execution_metadata"]["minimum_receive"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                > 0
        );
        count += 1;
        assert!(count <= 100, "bounded hot-reader profile");
        tokio::time::sleep(read_interval).await;
    }
    publisher.await.unwrap();
    println!("maximum_current_pairs=512 orders_per_pair=2048 total_orders=1048576 single_tokio_worker=true useful_http_quotes={count} max_ms={maximum_ms} publication_ms={}", publication_started.elapsed().as_millis());
    server.abort();
}

#[tokio::test]
#[serial_test::serial]
async fn issue1489_pending_admission_never_recreates_known_blacklist_approval() {
    let namespace = "late-blacklist-publication1489";
    let actor = address(30000);
    let mut source = tests::fixture();
    source.namespace = namespace.into();
    source.actors.insert(actor.clone(), (1000, false));
    source.discounted_pairs = source
        .pairs
        .iter()
        .map(|p| p.contract_address.clone())
        .collect();
    pending(source.clone()).scheduled().await.unwrap();
    let mut q = tests::request(10u128.pow(18));
    q.trader = Some(actor.clone());
    assert!(
        solve_local(namespace, &q).await.is_ok(),
        "positive precondition must execute"
    );
    let older_pending = pending(source);
    ingestion::publish_known_block(namespace, &actor).unwrap();
    assert!(
        solve_local(namespace, &q).await.is_err(),
        "known negative must immediately revoke quote"
    );
    let mut topology_q = q.clone();
    topology_q.amount_in = None;
    let topology_body = topology(namespace, &topology_q).unwrap().0;
    assert_eq!(topology_body["quote_kind"], "indexer_route_only");
    assert!(topology_body["estimated_amount_out"].is_null());
    assert!(topology_body["execution_metadata"].is_null());
    let progress = progress_local(namespace, &q).unwrap();
    assert_ne!(
        progress.quote_eligibility.as_deref(),
        Some("executable_quote")
    );
    older_pending.scheduled().await.unwrap();
    assert!(
        solve_local(namespace, &q).await.is_err(),
        "a pending old actor map must not recreate approval or discount"
    );
    let topology_body = topology(namespace, &topology_q).unwrap().0;
    assert_eq!(topology_body["quote_kind"], "indexer_route_only");
    assert!(topology_body["execution_metadata"].is_null());
    assert_ne!(
        progress_local(namespace, &q)
            .unwrap()
            .quote_eligibility
            .as_deref(),
        Some("executable_quote")
    );
    ingestion::publish_known_unblock(namespace, &actor).unwrap();
    assert!(
        solve_local(namespace, &q).await.is_ok(),
        "only explicit fresh unblock can clear the negative"
    );
    let old_blocked = {
        ingestion::publish_known_block(namespace, &actor).unwrap();
        let old = (*store().lock().unwrap().snapshots[namespace]).clone();
        pending(old)
    };
    ingestion::publish_known_unblock(namespace, &actor).unwrap();
    old_blocked.scheduled().await.unwrap();
    assert!(
        solve_local(namespace, &q).await.is_ok(),
        "stale blocked snapshot cannot reblock a fresh authoritative unblock"
    );
    assert_eq!(
        store().lock().unwrap().snapshots[namespace].actors[&actor],
        (0, false)
    );
    let mut g = store().lock().unwrap();
    g.snapshots.remove(namespace);
    g.early_actor_blocks.remove(namespace);
    g.actor_versions.remove(namespace);
}
