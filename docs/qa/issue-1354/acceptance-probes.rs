// Issue #1354 acceptance reproductions; only a fresh isolated QA child.
#[serial]
#[tokio::test]
async fn qa1354_partial_priced_volume_acceptance() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_events(&pool).await;
    let d = day0() + Duration::hours(2);
    insert_swap(&pool, seed.pair_id, seed.asset_0_id, seed.asset_1_id, d, "priced", Some("558.461005")).await;
    insert_swap(&pool, seed.pair_id, seed.asset_0_id, seed.asset_1_id, d, "unpriced", None).await;
    refresh(&pool).await;
    let server = TestServer::new(build_test_app(pool.clone()).await);
    let body: Value = server.get(&format!("/api/v1/defillama/daily?timestamp={}", day0().timestamp())).await.json();
    assert_usd(&body["volume_usd"], "558.461005");
}
#[serial]
#[tokio::test]
async fn qa1354_z_mixed_gem_transaction_pair_fee_acceptance() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_events(&pool).await;
    let gem = seed_gem_pair(&pool, seed.asset_1_id).await;
    let d = day0() + Duration::hours(2);
    insert_swap(&pool, seed.pair_id, seed.asset_0_id, seed.asset_1_id, d, "mixed", Some("10")).await;
    insert_swap(&pool, gem, seed.asset_1_id, seed.asset_1_id, d, "mixed", Some("999")).await;
    insert_fee(&pool, FeeSource::SwapAmm, seed.asset_1_id, d, "mixed", Some("1.5"), 0).await;
    sqlx::query("UPDATE protocol_fee_events SET pair_id = $1 WHERE tx_hash = 'mixed'").bind(seed.pair_id).execute(&pool).await.unwrap();
    refresh(&pool).await;
    let rows = daily_q::get_daily_fees(&pool, day0().date_naive()).await.unwrap();
    let row = rows.iter().find(|r| r.source == "swap_amm").unwrap();
    assert_eq!(row.amount_usd, bd("1.5"), "the ordinary hop must retain its pair-linked fee");
}
