//! #1211: immutable chain-position payer cohorts and aggregate-only API.
mod common;
use axum_test::TestServer;
use bech32::ToBase32;
use chrono::Utc;
use cl8y_dex_indexer::{
    config::Config,
    db::queries::{fee_cohorts, protocol_fees as fees},
    indexer::{
        fee_cohorts::{attribute_transaction, AttributionBudget},
        protocol_fees::{FeeEventDraft, FeeSource},
    },
    lcd::{LcdClient, TxResponse},
};
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::PgPool;
use wiremock::{
    matchers::{header, method, path_regex},
    Mock, MockServer, ResponseTemplate,
};

fn addr(byte: u8) -> String {
    bech32::encode("terra", [byte; 20].to_base32(), bech32::Variant::Bech32).unwrap()
}
fn cfg() -> Config {
    Config {
        fee_discount_address: Some(addr(1)),
        router_address: Some(addr(2)),
        wrap_mapper_address: Some(addr(3)),
        ust1_window_address: Some(addr(4)),
        factory_address: addr(5),
        community_token_launcher: Some(addr(6)),
        ..common::test_config()
    }
}
fn action(emitter: &str, name: &str, attrs: &[(&str, &str)]) -> Value {
    let mut values = vec![
        json!({"key":"_contract_address","value":emitter}),
        json!({"key":"action","value":name}),
    ];
    values.extend(attrs.iter().map(|(k, v)| json!({"key":k,"value":v})));
    json!({"type":"wasm","attributes":values})
}
fn tx(hash: &str, height: i64, events: Vec<Value>) -> TxResponse {
    serde_json::from_value(
        json!({"txhash":hash,"height":height.to_string(),"logs":[{"events":events}]}),
    )
    .unwrap()
}
async fn fee(
    pool: &PgPool,
    source: FeeSource,
    ordinal: i64,
    hash: &str,
    height: i64,
    asset: i32,
    pair: Option<i32>,
    usd: Option<i32>,
) -> i64 {
    fees::insert_fee_event(
        pool,
        &FeeEventDraft {
            block_height: height,
            block_timestamp: Utc::now(),
            tx_hash: hash.into(),
            source,
            ordinal,
            pair_id: pair,
            asset_id: asset,
            amount_raw: 1.into(),
            decimals: 6,
            fee_usd: usd.map(Into::into),
        },
    )
    .await
    .unwrap();
    sqlx::query_scalar("SELECT id FROM protocol_fee_events WHERE tx_hash=$1 AND source=$2 AND ordinal=$3 ORDER BY id DESC LIMIT 1").bind(hash).bind(source.as_str()).bind(ordinal).fetch_one(pool).await.unwrap()
}
async fn tag(pool: &PgPool, id: i64) -> (Option<String>, String) {
    sqlx::query_as("SELECT actor,cohort FROM protocol_fee_attributions WHERE fee_event_id=$1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}
fn lcd(url: &str) -> LcdClient {
    LcdClient::new(vec![url.into()], 100, 100, "columbus-5")
}
async fn baseline(
    pool: &PgPool,
    cfg: &Config,
    actor: &str,
    height: i64,
    registered: bool,
    tier: i16,
) {
    sqlx::query("INSERT INTO fee_registry_snapshots(registry,actor,block_height,registered,tier_id) VALUES($1,$2,$3,$4,$5)").bind(&cfg.fee_discount_address).bind(actor).bind(height).bind(registered).bind(tier).execute(pool).await.unwrap();
}

#[tokio::test]
#[serial]
async fn chain_order_registration_deregistration_and_replay_keep_history() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let cfg = cfg();
    let actor = addr(9);
    let registry = cfg.fee_discount_address.as_deref().unwrap();
    let mapper = cfg.wrap_mapper_address.as_deref().unwrap();
    baseline(&pool, &cfg, &actor, 99, false, 0).await;
    // Mutable trader state deliberately disagrees with the certified prior block.
    sqlx::query("INSERT INTO traders(address,tier_id,registered) VALUES($1,0,true)")
        .bind(&actor)
        .execute(&pool)
        .await
        .unwrap();
    let events = vec![
        action(mapper, "wrap", &[("sender", &actor)]),
        action(
            registry,
            "register_wallet",
            &[("wallet", &actor), ("tier_id", "0")],
        ),
        action(mapper, "wrap", &[("sender", &actor)]),
        action(registry, "deregister_wallet", &[("wallet", &actor)]),
        action(mapper, "wrap", &[("sender", &actor)]),
    ];
    let tx = tx("ORDER", 100, events);
    let mut ids = vec![];
    for ordinal in 0..3 {
        ids.push(
            fee(
                &pool,
                FeeSource::Wrap,
                ordinal,
                &tx.txhash,
                100,
                seed.asset_0_id,
                None,
                Some(1),
            )
            .await,
        );
    }
    attribute_transaction(
        &pool,
        &lcd("http://127.0.0.1:9"),
        &cfg,
        &tx,
        100,
        0,
        &mut AttributionBudget::default(),
    )
    .await
    .unwrap();
    assert_eq!(
        tag(&pool, ids[0]).await,
        (Some(actor.clone()), "retail".into())
    );
    assert_eq!(tag(&pool, ids[1]).await, (Some(actor.clone()), "mm".into()));
    assert_eq!(
        tag(&pool, ids[2]).await,
        (Some(actor.clone()), "retail".into())
    );
    sqlx::query("UPDATE traders SET registered=false,tier_id=0 WHERE address=$1")
        .bind(&actor)
        .execute(&pool)
        .await
        .unwrap();
    attribute_transaction(
        &pool,
        &lcd("http://127.0.0.1:9"),
        &cfg,
        &tx,
        100,
        0,
        &mut AttributionBudget::default(),
    )
    .await
    .unwrap();
    assert_eq!(tag(&pool, ids[1]).await.1, "mm");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM fee_registry_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
#[serial]
async fn historical_provider_height_guard_admits_unregistered_and_rejects_ignored_height() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let cfg = cfg();
    let actor = addr(10);
    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    Mock::given(method("GET"))
        .and(path_regex("/smart/"))
        .and(header("x-cosmos-block-height", "99"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-cosmos-block-height", "99")
                .set_body_json(json!({"data":{"registered":false}})),
        )
        .mount(&server)
        .await;
    let tx1 = tx(
        "HIST",
        100,
        vec![action(
            &cfg.factory_address,
            "create_pair",
            &[("sender", &actor), ("creation_fee_uluna", "1")],
        )],
    );
    let first = fee(
        &pool,
        FeeSource::PairCreation,
        0,
        "HIST",
        100,
        seed.asset_0_id,
        None,
        Some(2),
    )
    .await;
    attribute_transaction(
        &pool,
        &lcd(&server.uri()),
        &cfg,
        &tx1,
        100,
        0,
        &mut AttributionBudget::default(),
    )
    .await
    .unwrap();
    assert_eq!(tag(&pool, first).await.1, "retail");
    server.reset().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    Mock::given(method("GET"))
        .and(path_regex("/smart/"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-cosmos-block-height", "999")
                .set_body_json(json!({"data":{"registered":true,"tier_id":0}})),
        )
        .mount(&server)
        .await;
    let tx2 = tx(
        "WRONGHEIGHT",
        101,
        vec![action(
            &cfg.factory_address,
            "create_pair",
            &[("sender", &actor), ("creation_fee_uluna", "1")],
        )],
    );
    let second = fee(
        &pool,
        FeeSource::PairCreation,
        0,
        "WRONGHEIGHT",
        101,
        seed.asset_0_id,
        None,
        Some(2),
    )
    .await;
    attribute_transaction(
        &pool,
        &lcd(&server.uri()),
        &cfg,
        &tx2,
        101,
        0,
        &mut AttributionBudget::default(),
    )
    .await
    .unwrap();
    assert_eq!(tag(&pool, second).await.1, "unknown");
}

#[tokio::test]
#[serial]
async fn cohort_rollups_are_priced_only_additive_private_and_bounded() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    for (n, cohort, usd) in [
        (0, "retail", Some(3)),
        (1, "mm", Some(1)),
        (2, "unknown", None),
    ] {
        let id = fee(
            &pool,
            FeeSource::Wrap,
            n,
            &format!("ROLL{n}"),
            100,
            seed.asset_0_id,
            None,
            usd,
        )
        .await;
        sqlx::query(
            "INSERT INTO protocol_fee_attributions(fee_event_id,actor,cohort) VALUES($1,$2,$3)",
        )
        .bind(id)
        .bind(addr(9))
        .bind(cohort)
        .execute(&pool)
        .await
        .unwrap();
    }
    // Historical rows without any new attribution remain explicitly unknown.
    fee(
        &pool,
        FeeSource::Wrap,
        3,
        "LEGACY",
        100,
        seed.asset_0_id,
        None,
        None,
    )
    .await;
    fees::refresh_protocol_fees(&pool, true, false)
        .await
        .unwrap();
    cl8y_dex_indexer::api::reset_protocol_fees_cache();
    let server = TestServer::new(common::build_test_app(pool.clone()).await);
    for window in ["24h", "7d", "30d"] {
        let response = server
            .get(&format!("/api/v1/protocol/fees?window={window}"))
            .await;
        response.assert_status_ok();
        let value: Value = response.json();
        let cohorts = value["by_cohort"].as_array().unwrap();
        assert_eq!(cohorts.len(), 3);
        assert!(!value["by_cohort"].to_string().contains("terra1"));
        assert!(!value["by_cohort"].to_string().contains("actor"));
        assert_eq!(cohorts[0]["cohort"], "retail");
        assert_eq!(
            cohorts[0]["share_pct"]
                .as_str()
                .unwrap()
                .parse::<f64>()
                .unwrap(),
            75.0
        );
        assert_eq!(
            cohorts[1]["share_pct"]
                .as_str()
                .unwrap()
                .parse::<f64>()
                .unwrap(),
            25.0
        );
        assert!(cohorts[2]["amount_usd"].is_null());
        assert!(cohorts[2]["share_pct"].is_null());
        assert_eq!(cohorts[2]["event_count"], 2);
        assert!(value.get("by_source").is_some());
        assert!(value.get("by_token").is_some());
    }
    server
        .get("/api/v1/protocol/fees?window=garbage")
        .await
        .assert_status_bad_request();
    sqlx::query("DELETE FROM protocol_fee_events")
        .execute(&pool)
        .await
        .unwrap();
    fee_cohorts::refresh(&pool, "24h", Utc::now() - chrono::Duration::hours(24))
        .await
        .unwrap();
    for row in fee_cohorts::get(&pool, "24h").await.unwrap() {
        assert_eq!(row.amount_usd, Some(0.into()));
        assert_eq!(row.event_count, 0);
        assert_eq!(row.share_pct, None);
    }
}

#[tokio::test]
#[serial]
async fn authenticated_sources_use_payer_taker_and_trusted_router_with_spoof_resistance() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let cfg = cfg();
    let retail = addr(20);
    let mm = addr(21);
    let token = addr(22);
    let router = cfg.router_address.as_deref().unwrap();
    let mapper = cfg.wrap_mapper_address.as_deref().unwrap();
    let window = cfg.ust1_window_address.as_deref().unwrap();
    let launcher = cfg.community_token_launcher.as_deref().unwrap();
    baseline(&pool, &cfg, &retail, 99, false, 0).await;
    baseline(&pool, &cfg, &mm, 99, true, 0).await;
    let mut events = vec![
        action(&seed.pair_address, "swap", &[("trader", &mm)]),
        action(
            "terra1rogue",
            "place_limit_order",
            &[("owner", &mm), ("order_id", "88")],
        ),
    ];
    // A direct untrusted sender cannot spoof the registry subject via trader.
    events.push(action(
        &seed.pair_address,
        "swap",
        &[
            ("sender", &retail),
            ("trader", &mm),
            ("offer_asset", "uluna"),
            ("ask_asset", "uusd"),
            ("offer_amount", "100"),
            ("return_amount", "90"),
        ],
    ));
    events.push(action(
        &seed.pair_address,
        "swap",
        &[
            ("sender", router),
            ("trader", &mm),
            ("offer_asset", "uluna"),
            ("ask_asset", "uusd"),
            ("offer_amount", "200"),
            ("return_amount", "180"),
        ],
    ));
    events.push(action(
        &seed.pair_address,
        "swap",
        &[
            ("sender", router),
            ("offer_asset", "uluna"),
            ("ask_asset", "uusd"),
            ("offer_amount", "300"),
            ("return_amount", "270"),
        ],
    ));
    events.push(action(
        &seed.pair_address,
        "place_limit_order",
        &[("owner", &retail), ("order_id", "88")],
    ));
    events.push(action(mapper, "wrap", &[("sender", &mm)]));
    events.push(action(mapper, "unwrap", &[("sender", &mm)]));
    events.push(action(window, "deposit", &[("sender", &retail)]));
    events.push(action(window, "withdraw", &[]));
    events.push(action(
        &cfg.factory_address,
        "create_pair",
        &[("sender", &retail), ("creation_fee_uluna", "1")],
    ));
    let mut custom_spoof = action(
        "terra1rogue",
        "enable_feature",
        &[("sender", &mm), ("invoice", "1")],
    );
    custom_spoof["type"] = json!("wasm-limit_order_fill");
    events.push(custom_spoof);
    events.push(action(
        launcher,
        "create_token",
        &[("sender", &retail), ("sku_count", "1")],
    ));
    events.push(action(
        &token,
        "update_settings",
        &[("sender", &retail), ("invoice", "1")],
    ));
    let tx = tx("SOURCES", 100, events);
    for (index, actor, amount, output) in [
        (0, &retail, 100, 90),
        (1, &mm, 200, 180),
        (2, &retail, 300, 270),
    ] {
        let swapid:i64=sqlx::query_scalar("INSERT INTO swap_events(pair_id,block_height,block_timestamp,tx_hash,sender,offer_asset_id,ask_asset_id,offer_amount,return_amount,price,swap_index) VALUES($1,100,NOW(),'SOURCES',$2,$3,$4,$5,$6,1,$7) RETURNING id")
            .bind(seed.pair_id).bind(actor).bind(seed.asset_0_id).bind(seed.asset_1_id).bind(bigdecimal::BigDecimal::from(amount)).bind(bigdecimal::BigDecimal::from(output)).bind(index).fetch_one(&pool).await.unwrap();
        if index == 1 {
            sqlx::query("INSERT INTO limit_order_fills(pair_id,swap_event_id,block_height,block_timestamp,tx_hash,order_id,side,maker,price,token0_amount,token1_amount,commission_amount) VALUES($1,$2,100,NOW(),'SOURCES',77,'bid',$3,1,1,1,1)").bind(seed.pair_id).bind(swapid).bind(&retail).execute(&pool).await.unwrap();
        }
    }
    sqlx::query("INSERT INTO limit_order_placements(pair_id,block_height,block_timestamp,tx_hash,order_id,owner) VALUES($1,100,NOW(),'SOURCES',88,$2)").bind(seed.pair_id).bind(&retail).execute(&pool).await.unwrap();
    let mut ids = vec![];
    for (source, ordinal, pair, cohort, payer) in [
        (
            FeeSource::SwapAmm,
            0,
            Some(seed.pair_id),
            "retail",
            Some(retail.clone()),
        ),
        (FeeSource::SwapAmm, 2, Some(seed.pair_id), "unknown", None),
        (FeeSource::BookTake, 77, None, "mm", Some(mm.clone())),
        (
            FeeSource::LimitPlace,
            88,
            None,
            "retail",
            Some(retail.clone()),
        ),
        (FeeSource::Wrap, 0, None, "mm", Some(mm.clone())),
        (FeeSource::Unwrap, 1, None, "mm", Some(mm.clone())),
        (FeeSource::Ust1Mint, 0, None, "retail", Some(retail.clone())),
        (FeeSource::Ust1Redeem, 1, None, "unknown", None),
        (
            FeeSource::PairCreation,
            0,
            None,
            "retail",
            Some(retail.clone()),
        ),
        (
            FeeSource::SkuUnlock,
            0,
            None,
            "retail",
            Some(retail.clone()),
        ),
        (
            FeeSource::SettingsFee,
            0,
            None,
            "retail",
            Some(retail.clone()),
        ),
    ] {
        let id = fee(
            &pool,
            source,
            ordinal,
            &tx.txhash,
            100,
            seed.asset_0_id,
            pair,
            Some(1),
        )
        .await;
        ids.push((id, cohort, payer));
    }
    attribute_transaction(
        &pool,
        &lcd("http://127.0.0.1:9"),
        &cfg,
        &tx,
        100,
        0,
        &mut AttributionBudget::default(),
    )
    .await
    .unwrap();
    for (id, cohort, payer) in ids {
        assert_eq!(tag(&pool, id).await, (payer, cohort.into()), "fee {id}");
    }
}

#[tokio::test]
#[serial]
async fn self_and_blacklist_tiers_are_retail_and_forged_registry_is_ignored() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let cfg = cfg();
    let actor = addr(23);
    let registry = cfg.fee_discount_address.as_deref().unwrap();
    let mapper = cfg.wrap_mapper_address.as_deref().unwrap();
    baseline(&pool, &cfg, &actor, 99, false, 0).await;
    let tx = tx(
        "TIERS",
        100,
        vec![
            action(
                "terra1untrusted",
                "register_wallet",
                &[("wallet", &actor), ("tier_id", "0")],
            ),
            action(mapper, "wrap", &[("sender", &actor)]),
            action(
                registry,
                "register",
                &[("wallet", &actor), ("tier_id", "5")],
            ),
            action(mapper, "wrap", &[("sender", &actor)]),
            action(
                registry,
                "register_wallet",
                &[("wallet", &actor), ("tier_id", "255")],
            ),
            action(mapper, "wrap", &[("sender", &actor)]),
            action(mapper, "wrap", &[("sender", &actor), ("sender", &addr(24))]),
        ],
    );
    let mut ids = vec![];
    for ordinal in 0..4 {
        ids.push(
            fee(
                &pool,
                FeeSource::Wrap,
                ordinal,
                &tx.txhash,
                100,
                seed.asset_0_id,
                None,
                Some(1),
            )
            .await,
        );
    }
    attribute_transaction(
        &pool,
        &lcd("http://127.0.0.1:9"),
        &cfg,
        &tx,
        100,
        0,
        &mut AttributionBudget::default(),
    )
    .await
    .unwrap();
    for id in &ids[..3] {
        assert_eq!(tag(&pool, *id).await.1, "retail");
    }
    assert_eq!(tag(&pool, ids[3]).await, (None, "unknown".into()));
}

#[tokio::test]
#[serial]
async fn cohort_overflow_matches_source_nulls_and_publication_failure_rolls_back_all_rows() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let id = fee(
        &pool,
        FeeSource::Wrap,
        0,
        "INITIAL",
        100,
        seed.asset_0_id,
        None,
        Some(2),
    )
    .await;
    sqlx::query(
        "INSERT INTO protocol_fee_attributions(fee_event_id,actor,cohort) VALUES($1,$2,'retail')",
    )
    .bind(id)
    .bind(addr(9))
    .execute(&pool)
    .await
    .unwrap();
    fees::refresh_protocol_fees(&pool, true, false)
        .await
        .unwrap();
    let before_sources = fees::get_fees_by_source(&pool, "24h").await.unwrap();
    let before_cohorts = fee_cohorts::get(&pool, "24h").await.unwrap();
    fee(
        &pool,
        FeeSource::Wrap,
        1,
        "ADDED",
        100,
        seed.asset_0_id,
        None,
        Some(5),
    )
    .await;
    sqlx::query("CREATE FUNCTION reject_cohort_test_write() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic publication failure'; END $$").execute(&pool).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_cohort_test_write BEFORE INSERT OR UPDATE ON protocol_fee_stats_by_cohort FOR EACH ROW EXECUTE FUNCTION reject_cohort_test_write()").execute(&pool).await.unwrap();
    assert!(fees::refresh_protocol_fees(&pool, true, false)
        .await
        .is_err());
    let after_sources = fees::get_fees_by_source(&pool, "24h").await.unwrap();
    let after_cohorts = fee_cohorts::get(&pool, "24h").await.unwrap();
    assert_eq!(
        before_sources
            .iter()
            .find(|r| r.source == "wrap")
            .unwrap()
            .amount_usd,
        after_sources
            .iter()
            .find(|r| r.source == "wrap")
            .unwrap()
            .amount_usd
    );
    assert_eq!(before_cohorts[0].amount_usd, after_cohorts[0].amount_usd);
    sqlx::query("DROP TRIGGER reject_cohort_test_write ON protocol_fee_stats_by_cohort")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DROP FUNCTION reject_cohort_test_write()")
        .execute(&pool)
        .await
        .unwrap();
    fees::refresh_protocol_fees(&pool, true, false)
        .await
        .unwrap();
    let source = fees::get_fees_by_source(&pool, "24h").await.unwrap();
    assert_eq!(
        source
            .iter()
            .find(|r| r.source == "wrap")
            .unwrap()
            .amount_usd,
        Some(7.into())
    );
    sqlx::query("UPDATE protocol_fee_events SET fee_usd=90000000000000000000")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO protocol_fee_attributions(fee_event_id,actor,cohort) SELECT id,$1,'retail' FROM protocol_fee_events ON CONFLICT DO NOTHING").bind(addr(9)).execute(&pool).await.unwrap();
    fees::refresh_protocol_fees(&pool, true, false)
        .await
        .unwrap();
    let cohorts = fee_cohorts::get(&pool, "24h").await.unwrap();
    assert!(cohorts[0].amount_usd.is_none());
    assert!(cohorts[0].share_pct.is_none());
}

#[tokio::test]
#[serial]
async fn malformed_pinned_registry_receipts_invalidate_prior_membership_fail_closed() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let cfg = cfg();
    let actor = addr(25);
    let other = addr(26);
    let registry = cfg.fee_discount_address.as_deref().unwrap();
    let mapper = cfg.wrap_mapper_address.as_deref().unwrap();
    baseline(&pool, &cfg, &actor, 99, false, 0).await;
    baseline(&pool, &cfg, &other, 99, true, 0).await;
    let tx = tx(
        "MALFORMED",
        100,
        vec![
            action(
                registry,
                "register_wallet",
                &[("wallet", &actor), ("tier_id", "broken")],
            ),
            action(mapper, "wrap", &[("sender", &actor)]),
            action(registry, "deregister_wallet", &[]),
            action(mapper, "wrap", &[("sender", &other)]),
            action(
                registry,
                "register_wallet",
                &[("wallet", &other), ("tier_id", "5")],
            ),
            action(mapper, "wrap", &[("sender", &other)]),
        ],
    );
    let mut ids = vec![];
    for ordinal in 0..3 {
        ids.push(
            fee(
                &pool,
                FeeSource::Wrap,
                ordinal,
                &tx.txhash,
                100,
                seed.asset_0_id,
                None,
                Some(1),
            )
            .await,
        );
    }
    attribute_transaction(
        &pool,
        &lcd("http://127.0.0.1:9"),
        &cfg,
        &tx,
        100,
        0,
        &mut AttributionBudget::default(),
    )
    .await
    .unwrap();
    assert_eq!(tag(&pool, ids[0]).await.1, "unknown");
    assert_eq!(tag(&pool, ids[1]).await.1, "unknown");
    assert_eq!(tag(&pool, ids[2]).await.1, "retail");
}

#[tokio::test]
#[serial]
async fn cohort_http_dust_amounts_and_shares_use_plain_decimal_strings() {
    use std::str::FromStr;
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let dust = fee(
        &pool,
        FeeSource::Wrap,
        0,
        "DUST",
        100,
        seed.asset_0_id,
        None,
        Some(1),
    )
    .await;
    let large = fee(
        &pool,
        FeeSource::Wrap,
        1,
        "LARGE",
        100,
        seed.asset_0_id,
        None,
        Some(1),
    )
    .await;
    sqlx::query("UPDATE protocol_fee_events SET fee_usd=$1 WHERE id=$2")
        .bind(bigdecimal::BigDecimal::from_str("0.000000000000000001").unwrap())
        .bind(dust)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE protocol_fee_events SET fee_usd=99.999999999999999999 WHERE id=$1")
        .bind(large)
        .execute(&pool)
        .await
        .unwrap();
    for (id, cohort) in [(dust, "retail"), (large, "mm")] {
        sqlx::query(
            "INSERT INTO protocol_fee_attributions(fee_event_id,actor,cohort) VALUES($1,$2,$3)",
        )
        .bind(id)
        .bind(addr(9))
        .bind(cohort)
        .execute(&pool)
        .await
        .unwrap();
    }
    fees::refresh_protocol_fees(&pool, true, false)
        .await
        .unwrap();
    cl8y_dex_indexer::api::reset_protocol_fees_cache();
    let server = TestServer::new(common::build_test_app(pool).await);
    for window in ["24h", "7d", "30d"] {
        let response = server
            .get(&format!("/api/v1/protocol/fees?window={window}"))
            .await;
        response.assert_status_ok();
        let value: Value = response.json();
        let dust_row = value["by_cohort"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["cohort"] == "retail")
            .unwrap();
        assert_eq!(dust_row["amount_usd"], "0.000000000000000001");
        assert_eq!(dust_row["share_pct"], "0.000000000000000001");
    }
}
