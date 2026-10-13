//! Initialized same-cohort socket discriminator for #1489. Real anonymous GET,
//! POST, receive/progress handlers read admitted fixtures with providers disabled.
use super::*;
use bigdecimal::num_bigint::BigInt;
const UNIT: u128 = 1_000_000;
fn output(input: u128, rin: u128, rout: u128) -> u128 {
    let gross: u128 = (BigInt::from(input) * BigInt::from(rout) / BigInt::from(rin + input))
        .to_string()
        .parse()
        .unwrap();
    gross - gross * 30 / 10_000
}
async fn read(
    client: &reqwest::Client,
    base: &str,
    endpoint: &str,
    input: &str,
    amount: &str,
) -> Value {
    let started = Instant::now();
    let response = client
        .get(format!("{base}/api/v1/route/{endpoint}"))
        .query(&[
            ("token_in", input),
            ("token_out", USTR),
            ("amount_in", amount),
            ("max_spread", "0.05"),
            ("pool_only", "true"),
            ("protection_policy", "quote_relative_v1"),
        ])
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body: Value = response.json().await.unwrap();
    assert!(status.is_success(), "{status} {body}");
    assert!(
        started.elapsed().as_millis() <= 500,
        "initialized socket exceeded 500ms"
    );
    body
}
#[tokio::test]
#[serial_test::serial]
async fn issue1489_native_wrapped_get_post_receive_progress_share_original_best_cohort() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let clunc = asset(&pool, CLUNC, 6).await;
    let custc = asset(&pool, CUSTC, 6).await;
    let ust1 = asset(&pool, UST1, 18).await;
    let ustr = asset(&pool, USTR, 6).await;
    asset(&pool, "uluna", 6).await;
    let six = 1_000_000 * UNIT;
    let eighteen = 1_000_000 * 10u128.pow(18);
    pair(
        &pool,
        "terra1short1489",
        clunc,
        ustr,
        5_000 * UNIT,
        5_000 * UNIT,
    )
    .await;
    pair(&pool, "terra1first1489", clunc, custc, six, six).await;
    pair(&pool, "terra1middle1489", custc, ust1, six, eighteen).await;
    pair(&pool, "terra1last1489", ust1, ustr, eighteen, six).await;
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
    let schema: Value = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    for field in [
        "search_complete",
        "search_truncated",
        "search_max_pool_hops",
    ] {
        assert!(
            schema["components"]["schemas"]["RouteSolveResponse"]["properties"][field].is_object(),
            "OpenAPI missing {field}"
        );
    }
    for field in ["best_amount_out", "search_complete", "search_truncated"] {
        assert!(
            schema["components"]["schemas"]["SolveProgress"]["properties"][field].is_object(),
            "progress OpenAPI missing {field}"
        );
    }
    let gross = 10_000 * UNIT;
    let credit = gross - gross * 200 / 10_000;
    let expected = output(
        output(output(credit, six, six), six, eighteen),
        eighteen,
        six,
    );
    let initial = read(&client, &base, "solve", "uluna", &gross.to_string()).await;
    let generation = initial["execution_metadata"]["generation"].clone();
    let post=client.post(format!("{base}/api/v1/route/solve")).json(&json!({"token_in":"uluna","token_out":USTR,"amount_in":gross.to_string(),"max_spread":"0.05","protection_policy":"quote_relative_v1","pool_only":true,"hybrid_by_hop":[null,null,null]})).send().await.unwrap();
    let status = post.status();
    let declared: Value = post.json().await.unwrap();
    assert!(status.is_success(), "{status} {declared}");
    assert_eq!(declared["estimated_amount_out_net"], expected.to_string());
    let deadline = Instant::now();
    loop {
        let progress = read(
            &client,
            &base,
            "solve/progress",
            "uluna",
            &gross.to_string(),
        )
        .await;
        if progress["best_amount_out"] == expected.to_string() {
            break;
        }
        assert!(
            deadline.elapsed().as_secs_f64() < 3.0,
            "same-cohort native promotion exceeded 3s"
        );
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    for endpoint in ["solve", "solve/best"] {
        let body = read(&client, &base, endpoint, "uluna", &gross.to_string()).await;
        assert_eq!(body["estimated_amount_out_net"], expected.to_string());
        assert_eq!(body["hops"].as_array().unwrap().len(), 3);
        assert_eq!(body["execution_metadata"]["generation"], generation);
        assert_eq!(
            body["execution_metadata"]["wrapper"]["route_input_amount"],
            credit.to_string()
        );
        assert_eq!(body["search_complete"], true);
        assert_eq!(body["search_truncated"], false);
        assert_eq!(body["economic_attribution"]["status"], "available");
    }
    let wrapped = read(&client, &base, "solve", CLUNC, &credit.to_string()).await;
    let mut wrapped_best = wrapped;
    let deadline = Instant::now();
    while wrapped_best["estimated_amount_out_net"] != expected.to_string() {
        assert!(deadline.elapsed().as_secs_f64() < 3.0);
        tokio::task::yield_now().await;
        wrapped_best = read(&client, &base, "solve/best", CLUNC, &credit.to_string()).await;
    }
    // Equal gross wrapped has its own actual economics, not native credit or a
    // scaled cached quote. Supported receive minimizes exact input across all
    // evaluated paths (existing 64 inverse-path cap remains in force).
    let equal_expected = output(
        output(output(gross, six, six), six, eighteen),
        eighteen,
        six,
    );
    let mut equal = read(&client, &base, "solve", CLUNC, &gross.to_string()).await;
    let deadline = Instant::now();
    while equal["estimated_amount_out_net"] != equal_expected.to_string() {
        assert!(deadline.elapsed().as_secs_f64() < 3.0);
        tokio::task::yield_now().await;
        equal = read(&client, &base, "solve/best", CLUNC, &gross.to_string()).await;
    }
    let receive = client
        .get(format!("{base}/api/v1/route/solve"))
        .query(&[
            ("token_in", "uluna"),
            ("token_out", USTR),
            ("amount_out", &expected.to_string()),
            ("max_spread", "0.05"),
            ("pool_only", "true"),
            ("protection_policy", "quote_relative_v1"),
        ])
        .send()
        .await
        .unwrap();
    let status = receive.status();
    let receive: Value = receive.json().await.unwrap();
    assert!(status.is_success(), "{status} {receive}");
    assert_eq!(receive["hops"].as_array().unwrap().len(), 3);
    assert!(
        receive["required_amount_in"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            <= gross
    );
    assert_eq!(
        upstream.received_requests().await.unwrap().len(),
        0,
        "quote/progress triggered provider calls"
    );
    eprintln!("1489 immutable native gross={gross} credit={credit} expected={expected} equal_gross_wrapped={equal_expected} generation={generation}");
    server.abort();
}
