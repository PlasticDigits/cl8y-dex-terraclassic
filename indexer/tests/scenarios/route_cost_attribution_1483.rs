//! Independent same-cohort exact economics and amount-cache ranking for #1483.
//! No solver arithmetic helper contributes to the expected amounts below.
use super::*;
use bigdecimal::num_bigint::BigInt;

const UNIT: u128 = 1_000_000;
const DIRECT: &str = "terra1directcost14830000000000000000000";
const VIA_UST1: &str = "terra1ust1incost148300000000000000000000";
const VIA_CL8Y: &str = "terra1cl8yincost148300000000000000000000";

fn pool_output(offer: u128, rin: u128, rout: u128) -> u128 {
    let gross: u128 = (BigInt::from(offer) * BigInt::from(rout) / BigInt::from(rin + offer))
        .to_string()
        .parse()
        .unwrap();
    gross - gross * 30 / 10_000
}

fn candidates(offer: u128) -> [(u128, &'static str); 3] {
    let direct = pool_output(offer, 100 * UNIT, 120 * UNIT);
    let ust1 = pool_output(offer, 1_000_000 * UNIT, 1_000_000 * 10u128.pow(18));
    let ust1 = pool_output(ust1, 1_000_000 * 10u128.pow(18), 1_000_000 * UNIT);
    let cl8y = pool_output(offer, 1_000_000 * UNIT, 950_000 * 10u128.pow(18));
    let cl8y = pool_output(cl8y, 1_000_000 * 10u128.pow(18), 1_000_000 * UNIT);
    [(direct, DIRECT), (ust1, VIA_UST1), (cl8y, VIA_CL8Y)]
}

fn assert_attribution(body: &Value, gross: u128, credit: u128) {
    let winner = candidates(credit).into_iter().max_by_key(|c| c.0).unwrap();
    let reserves = match winner.1 {
        DIRECT => vec![(100 * UNIT, 120 * UNIT)],
        VIA_UST1 => vec![
            (1_000_000 * UNIT, 1_000_000 * 10u128.pow(18)),
            (1_000_000 * 10u128.pow(18), 1_000_000 * UNIT),
        ],
        VIA_CL8Y => vec![
            (1_000_000 * UNIT, 950_000 * 10u128.pow(18)),
            (1_000_000 * 10u128.pow(18), 1_000_000 * UNIT),
        ],
        _ => unreachable!(),
    };
    let project = |mut raw: u128, suffix: &[(u128, u128)]| {
        for (rin, rout) in suffix {
            raw = (BigInt::from(raw) * BigInt::from(*rout) / BigInt::from(*rin))
                .to_string()
                .parse()
                .unwrap();
        }
        raw
    };
    let spot = project(gross, &reserves);
    let wrapper = spot - project(credit, &reserves);
    let mut trading = 0u128;
    let mut movement = 0u128;
    let mut input = credit;
    for (index, (rin, rout)) in reserves.iter().enumerate() {
        let raw: u128 = (BigInt::from(input) * BigInt::from(*rout) / BigInt::from(rin + input))
            .to_string()
            .parse()
            .unwrap();
        let net = raw - raw * 30 / 10_000;
        let before = project(raw, &reserves[index + 1..]);
        trading += before - project(net, &reserves[index + 1..]);
        movement += project(input, &reserves[index..]) - before;
        input = net;
    }
    let attr = &body["economic_attribution"];
    assert_eq!(attr["status"], "available");
    assert_eq!(attr["fair_amount_out"], gross.to_string());
    assert_eq!(attr["net_amount_out"], input.to_string());
    assert_eq!(attr["components"]["wrapper_loss"], wrapper.to_string());
    assert_eq!(attr["components"]["trading_cost"], trading.to_string());
    assert_eq!(attr["components"]["market_movement"], movement.to_string());
    assert_eq!(
        attr["components"]["reference_divergence"],
        (BigInt::from(gross) - BigInt::from(spot)).to_string()
    );
    for field in ["generation", "observed_at_ms", "expires_at_ms"] {
        assert_eq!(attr[field], body["execution_metadata"][field]);
    }
}

async fn quote(
    client: &reqwest::Client,
    base: &str,
    endpoint: &str,
    input: &str,
    amount: u128,
) -> (Value, f64) {
    let started = Instant::now();
    let response = client
        .get(format!("{base}/api/v1/route/{endpoint}"))
        .query(&[
            ("token_in", input),
            ("token_out", USTR),
            ("amount_in", &amount.to_string()),
            ("max_spread", "0.30"),
            ("pool_only", "true"),
            ("protection_policy", "quote_relative_v1"),
        ])
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert!(
        status.is_success(),
        "{endpoint} {input} {amount}: {status} {body}"
    );
    let body: Value = serde_json::from_str(&body).unwrap();
    let elapsed = started.elapsed().as_secs_f64() * 1_000.0;
    assert!(
        elapsed <= 500.0,
        "initialized useful socket >500ms: {elapsed}"
    );
    (body, elapsed)
}

#[tokio::test]
#[serial_test::serial]
async fn issue1483_exact_amount_cache_ranks_same_cohort_native_and_wrapped_candidates() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let clunc = asset(&pool, CLUNC, 6).await;
    let ustr = asset(&pool, USTR, 6).await;
    let ust1 = asset(&pool, UST1, 18).await;
    let cl8y = asset(&pool, CL8Y, 18).await;
    asset(&pool, "uluna", 6).await;
    pair(&pool, DIRECT, clunc, ustr, 100 * UNIT, 120 * UNIT).await;
    pair(
        &pool,
        VIA_UST1,
        clunc,
        ust1,
        1_000_000 * UNIT,
        1_000_000 * 10u128.pow(18),
    )
    .await;
    pair(
        &pool,
        "terra1ust1outcost14830000000000000000000",
        ust1,
        ustr,
        1_000_000 * 10u128.pow(18),
        1_000_000 * UNIT,
    )
    .await;
    pair(
        &pool,
        VIA_CL8Y,
        clunc,
        cl8y,
        1_000_000 * UNIT,
        950_000 * 10u128.pow(18),
    )
    .await;
    pair(
        &pool,
        "terra1cl8youtcost14830000000000000000000",
        cl8y,
        ustr,
        1_000_000 * 10u128.pow(18),
        1_000_000 * UNIT,
    )
    .await;
    let upstream = common::lcd_mock::start_ordinary_code_mock().await;
    mount_supported_config_mock(&upstream, 200).await;
    let mut state = app_state(pool.clone(), cold_fixture_lcd(upstream.uri()));
    state.wrap_mapper_address = Some("terra1mapperfixture1452".into());
    seed_reference_marks(&pool, 1).await;
    cl8y_dex_indexer::api::local_quote::ingest_fixture(&state)
        .await
        .unwrap();
    sqlx::query("UPDATE pair_reserves SET snapshot_at=NOW()")
        .execute(&pool)
        .await
        .unwrap();
    seed_reference_marks(&pool, 1).await;
    cl8y_dex_indexer::api::local_quote::refresh_reserves(&state)
        .await
        .unwrap();
    assert!(cl8y_dex_indexer::api::local_quote::refresh_wrapper(&state).await);
    upstream.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&upstream)
        .await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, build_router(state, &common::test_config()))
            .await
            .unwrap()
    });
    let client = reqwest::Client::new();
    let mut timing = Vec::new();
    let mut generation = None;
    // Warm the small amount before requesting a large one, then reverse order.
    // A cached winning skeleton must never suppress amount-specific alternatives.
    for amount in [UNIT, 1_000 * UNIT, UNIT, 4_000 * UNIT, 100 * UNIT] {
        let expected = candidates(amount)
            .into_iter()
            .max_by_key(|(net, _)| *net)
            .unwrap();
        for endpoint in ["solve", "solve/best"] {
            let (body, elapsed) = quote(&client, &base, endpoint, CLUNC, amount).await;
            timing.push(elapsed);
            assert_eq!(body["estimated_amount_out_net"], expected.0.to_string());
            assert_eq!(body["hops"][0]["pair"], expected.1);
            assert_attribution(&body, amount, amount);
            let current = body["execution_metadata"]["generation"].clone();
            if let Some(original) = &generation {
                assert_eq!(&current, original);
            } else {
                generation = Some(current);
            }
            let floor = (expected.0 * 7 + 9) / 10;
            assert_eq!(
                body["execution_metadata"]["minimum_receive"],
                floor.to_string()
            );
        }
        let response = client
            .post(format!("{base}/api/v1/route/solve"))
            .json(
                &json!({"token_in":CLUNC,"token_out":USTR,"amount_in":amount.to_string(),
                "max_spread":"0.30","protection_policy":"quote_relative_v1"}),
            )
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body: Value = response.json().await.unwrap();
        assert!(status.is_success(), "POST {amount}: {status} {body}");
        assert_eq!(body["estimated_amount_out_net"], expected.0.to_string());
        assert_eq!(
            body["execution_metadata"]["generation"],
            *generation.as_ref().unwrap()
        );
    }
    assert_eq!(
        candidates(UNIT).into_iter().max_by_key(|c| c.0).unwrap().1,
        DIRECT
    );
    assert_eq!(
        candidates(1_000 * UNIT)
            .into_iter()
            .max_by_key(|c| c.0)
            .unwrap()
            .1,
        VIA_UST1
    );
    for gross in [UNIT, 100 * UNIT, 1_000 * UNIT, 4_000 * UNIT] {
        let credit = gross - gross * 200 / 10_000;
        let (native, elapsed) = quote(&client, &base, "solve", "uluna", gross).await;
        timing.push(elapsed);
        let (matched, _) = quote(&client, &base, "solve", CLUNC, credit).await;
        let (equal, _) = quote(&client, &base, "solve", CLUNC, gross).await;
        assert_eq!(
            native["estimated_amount_out_net"],
            matched["estimated_amount_out_net"]
        );
        assert_attribution(&native, gross, credit);
        assert_attribution(&matched, credit, credit);
        assert_attribution(&equal, gross, gross);
        assert_eq!(
            native["execution_metadata"]["minimum_receive"],
            matched["execution_metadata"]["minimum_receive"]
        );
        assert_eq!(
            native["execution_metadata"]["wrapper"]["route_input_amount"],
            credit.to_string()
        );
        assert_eq!(
            native["execution_metadata"]["input_amount"],
            gross.to_string()
        );
        assert_eq!(
            native["estimated_amount_out_net"],
            candidates(credit)
                .into_iter()
                .map(|c| c.0)
                .max()
                .unwrap()
                .to_string()
        );
        assert!(
            equal["estimated_amount_out_net"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                > native["estimated_amount_out_net"]
                    .as_str()
                    .unwrap()
                    .parse::<u128>()
                    .unwrap()
        );
    }
    // Independently scan exact offers around the positive receive target instead
    // of trusting the inverse's own forward helper or a scaled cached quote.
    let target = candidates(1_000 * UNIT)
        .into_iter()
        .map(|c| c.0)
        .max()
        .unwrap();
    let response = client
        .get(format!("{base}/api/v1/route/solve"))
        .query(&[
            ("token_in", CLUNC),
            ("token_out", USTR),
            ("amount_out", &target.to_string()),
            ("max_spread", "0.30"),
            ("pool_only", "true"),
            ("protection_policy", "quote_relative_v1"),
        ])
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body: Value = response.json().await.unwrap();
    assert!(status.is_success(), "receive: {status} {body}");
    let offer = body["execution_metadata"]["input_amount"]
        .as_str()
        .unwrap()
        .parse::<u128>()
        .unwrap();
    assert!(candidates(offer).into_iter().map(|c| c.0).max().unwrap() >= target);
    assert!(
        candidates(offer - 1)
            .into_iter()
            .map(|c| c.0)
            .max()
            .unwrap()
            < target
    );
    assert_eq!(
        body["execution_metadata"]["generation"],
        *generation.as_ref().unwrap()
    );
    assert_eq!(
        upstream.received_requests().await.unwrap().len(),
        0,
        "quote requests must not poll LCD"
    );
    report(
        "1483 same-cohort initialized useful socket budget=500ms",
        &mut timing,
    );
    server.abort();
}

#[tokio::test]
#[serial_test::serial]
async fn issue1483_deeper_cache_publishes_every_strict_net_improvement_below_one_permille() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let clunc = asset(&pool, CLUNC, 6).await;
    let ustr = asset(&pool, USTR, 6).await;
    let ust1 = asset(&pool, UST1, 18).await;
    let cl8y = asset(&pool, CL8Y, 18).await;
    let six = 1_000_000 * UNIT;
    let eighteen = 1_000_000 * 10u128.pow(18);
    let baseline = pool_output(UNIT, six, six);
    let target = baseline + baseline / 2_000;
    let middle = pool_output(pool_output(UNIT, six, eighteen), eighteen, eighteen);
    // Choose original last-hop reserves by independent integer search so the
    // three-hop path improves the direct route by exactly ~0.05%, below the
    // old undocumented 0.1% cache-publishing threshold.
    let (mut low, mut high) = (six, six * 2);
    while low < high {
        let candidate = low + (high - low) / 2;
        if pool_output(middle, eighteen, candidate) >= target {
            high = candidate;
        } else {
            low = candidate + 1;
        }
    }
    let expected = pool_output(middle, eighteen, low);
    assert!(expected > baseline && expected - baseline <= baseline / 1_000);
    pair(&pool, DIRECT, clunc, ustr, six, six).await;
    pair(&pool, VIA_UST1, clunc, ust1, six, eighteen).await;
    pair(
        &pool,
        "terra1tinybridgecost14830000000000000000",
        ust1,
        cl8y,
        eighteen,
        eighteen,
    )
    .await;
    pair(
        &pool,
        "terra1tinyoutcost14830000000000000000000",
        cl8y,
        ustr,
        eighteen,
        low,
    )
    .await;
    let addresses: std::collections::HashMap<_, _> = assets::get_all_assets(&pool)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|a| a.contract_address.map(|address| (a.id, address)))
        .collect();
    let rows = pairs::get_all_pairs(&pool).await.unwrap();
    let names: Vec<_> = rows.iter().map(|p| p.contract_address.clone()).collect();
    let mirrors = cl8y_dex_indexer::api::db_orderbook_sim::preload_pool_mirrors_for_pairs(
        &pool, &names, &addresses, 20_000,
    )
    .await
    .unwrap();
    let upstream = common::lcd_mock::start_ordinary_code_mock().await;
    let state = app_state(pool, cold_fixture_lcd(upstream.uri()));
    cl8y_dex_indexer::api::local_quote::initialize_for_test(
        state.lcd.chain_cache_namespace(),
        rows,
        addresses,
        mirrors,
        20_000,
    );
    upstream.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&upstream)
        .await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, build_router(state, &common::test_config()))
            .await
            .unwrap()
    });
    let client = reqwest::Client::new();
    let (initial, _) = quote(&client, &base, "solve", CLUNC, UNIT).await;
    assert_eq!(initial["estimated_amount_out_net"], baseline.to_string());
    assert_eq!(initial["hops"].as_array().unwrap().len(), 1);
    let mut improved = None;
    for _ in 0..100 {
        let (body, _) = quote(&client, &base, "solve", CLUNC, UNIT).await;
        if body["estimated_amount_out_net"] == expected.to_string() {
            improved = Some(body);
            break;
        }
        tokio::task::yield_now().await;
    }
    let improved = improved.expect(
        "every bounded evaluated strictly better net candidate must be published, even below 0.1%",
    );
    assert_eq!(improved["hops"].as_array().unwrap().len(), 3);
    assert_eq!(
        improved["execution_metadata"]["generation"],
        initial["execution_metadata"]["generation"]
    );
    assert_eq!(upstream.received_requests().await.unwrap().len(), 0);
    eprintln!(
        "1483 exact same-cohort deeper baseline={baseline} improved={expected} raw_difference={}",
        expected - baseline
    );
    server.abort();
}
