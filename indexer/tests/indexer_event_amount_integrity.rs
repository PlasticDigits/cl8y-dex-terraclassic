//! #1395: chain amounts survive ingestion exactly; malformed siblings cannot poison a block.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use cl8y_dex_indexer::db::queries::state;
use cl8y_dex_indexer::indexer::{block_indexer, oracle, parser::process_block_txs};
use cl8y_dex_indexer::lcd::{LcdClient, TxResponse};
use serde_json::{json, Value};
use serial_test::serial;
use sqlx::PgPool;
use wiremock::matchers::{method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PAIR: &str = "terra1amountintegritypair";
const PAIR_2: &str = "terra1amountintegritypairtwo";
const TRADER: &str = "terra1amountintegritytrader";
const MAX_RAW: &str = "99999999999999999999999999999999999999";
const MAX_PRICE: &str = "99999999999999999999.999999999999999999";
const MAPPER: &str = "terra1amountintegritymapper";
const WINDOW: &str = "terra1amountintegritywindow";

fn time() -> DateTime<Utc> {
    cl8y_dex_indexer::indexer::defillama::utc_day_start(Utc::now()) - chrono::Duration::hours(12)
}

fn event(contract: &str, action: &str, fields: &[(&str, &str)]) -> Value {
    let mut attrs = vec![
        json!({"key": "_contract_address", "value": contract}),
        json!({"key": "action", "value": action}),
    ];
    attrs.extend(
        fields
            .iter()
            .map(|(key, value)| json!({"key": key, "value": value})),
    );
    json!({"type": "wasm", "attributes": attrs})
}

fn swap(pair: &str, offer: &str, ret: &str, extras: &[(&str, &str)]) -> Value {
    let mut fields = vec![
        ("sender", TRADER),
        ("offer_asset", "uluna"),
        ("ask_asset", "uusd"),
        ("offer_amount", offer),
        ("return_amount", ret),
    ];
    fields.extend_from_slice(extras);
    event(pair, "swap", &fields)
}

fn tx(hash: &str, events: Vec<Value>) -> TxResponse {
    serde_json::from_value(json!({
        "height": "1395", "txhash": hash, "timestamp": time().to_rfc3339(),
        "logs": [{"events": events}]
    }))
    .unwrap()
}

fn lcd(server: &MockServer) -> LcdClient {
    LcdClient::new(vec![server.uri()], 1000, 1)
}

async fn seed(pool: &PgPool) -> i32 {
    common::clean_db(pool).await;
    sqlx::query("TRUNCATE indexer_state, indexer_failed_blocks")
        .execute(pool)
        .await
        .unwrap();
    let a: i32 = sqlx::query_scalar(
        "INSERT INTO assets (denom,is_cw20,name,symbol,decimals) VALUES ('uluna',false,'Luna','LUNC',6) RETURNING id",
    ).fetch_one(pool).await.unwrap();
    let b: i32 = sqlx::query_scalar(
        "INSERT INTO assets (denom,is_cw20,name,symbol,decimals) VALUES ('uusd',false,'USTC','USTC',6) RETURNING id",
    ).fetch_one(pool).await.unwrap();
    let pair: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address,asset_0_id,asset_1_id,lp_token,fee_bps) VALUES ($1,$2,$3,'terra1integritylp',30) RETURNING id",
    ).bind(PAIR).bind(a).bind(b).fetch_one(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO pairs (contract_address,asset_0_id,asset_1_id,lp_token,fee_bps) VALUES ($1,$2,$3,'terra1integritylptwo',30)",
    ).bind(PAIR_2).bind(a).bind(b).execute(pool).await.unwrap();
    pair
}

async fn ingest(pool: &PgPool, server: &MockServer, txs: &[TxResponse]) {
    let mut config = common::test_config();
    config.wrap_mapper_address = Some(MAPPER.into());
    config.ust1_window_address = Some(WINDOW.into());
    process_block_txs(
        pool,
        &lcd(server),
        &config,
        txs,
        1395,
        time(),
        &oracle::new_shared_price(),
    )
    .await
    .expect("malformed chain amounts are event-local failures");
}

async fn mount_block(server: &MockServer, height: i64, hash: &str, txs: &[TxResponse]) {
    // LCD response types intentionally implement Deserialize only.
    let tx_responses: Vec<Value> = txs
        .iter()
        .map(|tx| {
            let logs: Vec<Value> = tx
                .logs
                .iter()
                .flatten()
                .map(|log| {
                    let events: Vec<Value> = log
                        .events
                        .iter()
                        .map(|event| {
                            let attributes: Vec<Value> = event
                                .attributes
                                .iter()
                                .map(|a| json!({"key": a.key, "value": a.value}))
                                .collect();
                            json!({"type": event.event_type, "attributes": attributes})
                        })
                        .collect();
                    json!({"events": events})
                })
                .collect();
            json!({"height": height.to_string(), "txhash": tx.txhash,
            "timestamp": tx.timestamp, "logs": logs})
        })
        .collect();
    Mock::given(method("GET"))
        .and(path(format!(
            "/cosmos/base/tendermint/v1beta1/blocks/{height}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "block_id": {"hash": hash},
            "block": {"header": {"height": height.to_string(),"time": time().to_rfc3339()}}
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cosmos/tx/v1beta1/txs"))
        .and(query_param("query", format!("tx.height={height}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tx_responses": tx_responses, "pagination": {"total": txs.len().to_string()}
        })))
        .mount(server)
        .await;
}

#[tokio::test]
#[serial]
async fn mixed_amounts_reconcile_rows_fees_positions_liquidity_and_daily_evidence() {
    let pool = common::setup_pool().await;
    let pair = seed(&pool).await;
    let server = MockServer::start().await;
    let mut events = vec![swap(
        PAIR,
        "1000000",
        "2000000",
        &[("commission_amount", "30")],
    )];
    // Each malformed event sits between valid siblings in one transaction. PostgreSQL
    // must never turn these strings into zero, rounded integers or absent optionals.
    for bad in [
        "",
        "garbage",
        "-1",
        "+1",
        "1.5",
        "1e3",
        " 1",
        "1 ",
        "NaN",
        "1e999999",
        "100000000000000000000000000000000000000",
    ] {
        events.push(swap(PAIR, bad, "2000000", &[]));
    }
    for field in [
        "spread_amount",
        "commission_amount",
        "pool_return_amount",
        "book_return_amount",
        "limit_book_offer_consumed",
        "reserve_0",
        "reserve_1",
        "effective_fee_bps",
    ] {
        events.push(swap(PAIR, "1000000", "2000000", &[(field, "broken")]));
    }
    events.push(swap(PAIR, "3000000", "6000000", &[]));
    events.push(swap(
        PAIR_2,
        "4000000",
        "8000000",
        &[("commission_amount", "40")],
    ));
    events.extend([
        event(
            PAIR,
            "provide_liquidity",
            &[
                ("sender", TRADER),
                ("assets", "uluna 10, uusd 20"),
                ("share", "5"),
            ],
        ),
        event(
            PAIR,
            "withdraw_liquidity",
            &[
                ("sender", TRADER),
                ("refund_assets", "uluna broken, uusd 20"),
                ("withdrawn_share", "5"),
            ],
        ),
        event(
            PAIR_2,
            "provide_liquidity",
            &[
                ("sender", TRADER),
                ("assets", "uluna 10, uusd 20"),
                ("share", "1.5"),
            ],
        ),
        event(
            "terra1hook",
            "after_swap_burn",
            &[("burn_amount", "broken"), ("burn_token", "uluna")],
        ),
        event(
            "terra1hook",
            "after_swap_tax",
            &[("tax_amount", "7"), ("tax_token", "uusd")],
        ),
        event(
            "terra1hook",
            "after_swap_skip",
            &[("skipped", "no balance")],
        ),
        event(
            PAIR,
            "place_limit_order",
            &[
                ("order_id", "7"),
                ("owner", TRADER),
                ("side", "bid"),
                ("price", "1.123456789012345678"),
                ("maker_fee_amount", "9"),
            ],
        ),
        event(
            PAIR,
            "place_limit_order",
            &[
                ("order_id", "8"),
                ("owner", TRADER),
                ("side", "bid"),
                ("price", "2"),
                ("maker_fee_amount", "1.5"),
            ],
        ),
        event(
            PAIR,
            "limit_order_fill",
            &[
                ("order_id", "7"),
                ("side", "bid"),
                ("maker", TRADER),
                ("price", "1.123456789012345678"),
                ("token0_amount", "10"),
                ("token1_amount", "20"),
                ("commission_amount", "2"),
                ("swap_index", "0"),
            ],
        ),
        event(
            PAIR,
            "limit_order_fill",
            &[
                ("order_id", "8"),
                ("side", "bid"),
                ("maker", TRADER),
                ("price", "2"),
                ("token0_amount", "broken"),
                ("token1_amount", "20"),
                ("commission_amount", "2"),
            ],
        ),
        event(
            PAIR,
            "limit_order_expired_parked",
            &[
                ("order_id", "7"),
                ("maker", TRADER),
                ("side", "bid"),
                ("remaining", "1.5"),
            ],
        ),
        event(MAPPER, "notify_deposit", &[("fee", "5"), ("denom", "uusd")]),
        event(MAPPER, "unwrap", &[("fee", "1.5"), ("denom", "uusd")]),
        event(
            WINDOW,
            "deposit",
            &[("fee_amount", "1.5"), ("fee_asset", "uusd")],
        ),
        event(
            WINDOW,
            "deposit",
            &[("fee_amount", "11"), ("fee_asset", "uusd")],
        ),
    ]);
    let txs = [tx("mixed1395", events)];
    ingest(&pool, &server, &txs).await;
    ingest(&pool, &server, &txs).await;

    let swaps: Vec<(String,i32,BigDecimal,Option<BigDecimal>)> = sqlx::query_as(
        "SELECT p.contract_address,se.swap_index,se.offer_amount,se.commission_amount FROM swap_events se JOIN pairs p ON p.id=se.pair_id ORDER BY se.id",
    ).fetch_all(&pool).await.unwrap();
    assert_eq!(swaps.len(), 3);
    assert_eq!(swaps[0].1, 0);
    assert_eq!(
        swaps[1].1, 20,
        "rejected siblings still consume source ordinals"
    );
    assert_eq!(swaps[1].2, BigDecimal::from(3_000_000));
    assert!(swaps[1].3.is_none(), "absent commission remains NULL");
    assert_eq!(swaps[2].0, PAIR_2);
    assert_eq!(swaps[2].1, 0);
    let fees: Vec<(String,Option<i32>,i64,BigDecimal)> = sqlx::query_as(
        "SELECT source,pair_id,ordinal,amount_raw FROM protocol_fee_events ORDER BY source,pair_id,ordinal",
    ).fetch_all(&pool).await.unwrap();
    assert_eq!(
        fees.len(),
        6,
        "two AMM hops, book take, placement, wrap and window exactly once"
    );
    let amm: Vec<_> = fees.iter().filter(|row| row.0 == "swap_amm").collect();
    assert_eq!(amm.len(), 2);
    assert_ne!(
        amm[0].1, amm[1].1,
        "identical hop ordinal preserves pair fee identity"
    );
    let position_count: i32 = sqlx::query_scalar(
        "SELECT trade_count FROM trader_positions WHERE pair_id=$1 AND trader_address=$2",
    )
    .bind(pair)
    .bind(TRADER)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        position_count, 2,
        "malformed and replayed swaps cannot affect positions"
    );
    for (table, count) in [
        ("liquidity_events", 1_i64),
        ("hook_events", 2),
        ("limit_order_placements", 1),
        ("limit_order_fills", 1),
    ] {
        let actual: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(actual, count, "{table}");
    }
    let lifecycle: (String, Option<BigDecimal>) = sqlx::query_as(
        "SELECT lifecycle_status,remaining_escrow FROM limit_order_placements WHERE order_id=7",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(lifecycle.0, "active");
    assert!(
        lifecycle.1.is_none(),
        "malformed escrow must not mark an order parked"
    );
    cl8y_dex_indexer::db::queries::protocol_volume::refresh_protocol_daily(&pool)
        .await
        .unwrap();
    cl8y_dex_indexer::db::queries::protocol_fee_series::refresh_protocol_daily(&pool)
        .await
        .unwrap();
    cl8y_dex_indexer::api::reset_protocol_volume_cache();
    cl8y_dex_indexer::api::reset_protocol_fee_series_cache();
    let http = TestServer::new(common::build_test_app(pool.clone()).await);
    let day = time().format("%Y-%m-%d").to_string();
    let response = http.get(&format!("/api/v1/evidence/daily?day={day}")).await;
    response.assert_status_ok();
    let body: Value = response.json();
    let rows = body["events"].as_array().unwrap();
    assert_eq!(rows.iter().filter(|e| e["surface"] == "swap").count(), 3);
    assert_eq!(rows.iter().filter(|e| e["surface"] == "lp").count(), 1);
    assert_eq!(rows.iter().filter(|e| e["surface"] == "wrap").count(), 1);
    let mut amounts: Vec<_> = rows
        .iter()
        .filter(|e| e["surface"] == "swap")
        .map(|e| e["offer_amount"].as_str().unwrap())
        .collect();
    amounts.sort_unstable();
    assert_eq!(amounts, vec!["1000000", "3000000", "4000000"]);
    assert!(
        !response.text().contains(TRADER),
        "existing evidence export remains redacted"
    );
    let volume_response = http
        .get("/api/v1/protocol/volume/daily?grain=daily&limit=2")
        .await;
    volume_response.assert_status_ok();
    let volume: Value = volume_response.json();
    let volume_day = volume["series"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["utc_day"] == day)
        .expect("volume bucket");
    assert_eq!(
        volume_day["trade_count"], 3,
        "rollup count reconciles accepted evidence"
    );
    let fee_response = http
        .get("/api/v1/protocol/fees/daily?grain=daily&limit=2")
        .await;
    fee_response.assert_status_ok();
    let fee_series: Value = fee_response.json();
    let fee_day = fee_series["series"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["utc_day"] == day)
        .expect("fee bucket");
    assert_eq!(
        fee_day["event_count"], 6,
        "treasury rollup counts exactly accepted fee rows"
    );
}

#[tokio::test]
#[serial]
async fn poison_replays_advance_checkpoint_and_next_block_without_retry_failure() {
    let pool = common::setup_pool().await;
    seed(&pool).await;
    let server = MockServer::start().await;
    state::set_indexer_checkpoint(&pool, 1394, "prev1394")
        .await
        .unwrap();
    let txs = [tx(
        "poison1395",
        vec![
            swap(PAIR, "1e999999", "1", &[]),
            swap(PAIR, "1", "1", &[("commission_amount", "1.5")]),
            swap(PAIR, "1", MAX_RAW, &[]),
        ],
    )];
    mount_block(&server, 1395, "hash1395", &txs).await;
    mount_block(&server, 1396, "hash1396", &[]).await;
    let config = common::test_config();
    let client = lcd(&server);
    let price = oracle::new_shared_price();
    for _ in 0..2 {
        block_indexer::index_block_with_retries(&pool, &client, &config, 1395, &price)
            .await
            .unwrap();
        assert_eq!(
            state::get_indexer_checkpoint(&pool).await.unwrap(),
            (1395, Some("hash1395".into()))
        );
    }
    block_indexer::index_block(&pool, &client, &config, 1396, &price)
        .await
        .unwrap();
    assert_eq!(state::get_last_indexed_height(&pool).await.unwrap(), 1396);
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM swap_events),(SELECT count(*) FROM indexer_failed_blocks)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(counts, (0, 0));
}

#[tokio::test]
#[serial]
async fn infrastructure_failure_holds_checkpoint_then_retry_preserves_exactly_once_effects() {
    let pool = common::setup_pool().await;
    let pair = seed(&pool).await;
    let server = MockServer::start().await;
    let unknown = "terra1unknownamountintegritytokenxxxxxxxxxxxxxxxxxxxxxxxx";
    let txs = [tx(
        "retry1395",
        vec![
            swap(PAIR, "1000000", "2000000", &[("commission_amount", "30")]),
            swap(
                PAIR,
                "3000000",
                "6000000",
                &[("ask_asset", unknown), ("commission_amount", "40")],
            ),
        ],
    )];
    mount_block(&server, 1395, "hash1395", &txs).await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(ResponseTemplate::new(502).set_body_string("LCD unavailable"))
        .mount(&server)
        .await;
    state::set_indexer_checkpoint(&pool, 1394, "prev1394")
        .await
        .unwrap();
    let mut config = common::test_config();
    config.block_process_max_retries = 2;
    config.block_process_retry_backoff_ms = 1;
    let client = lcd(&server);
    let price = oracle::new_shared_price();
    assert!(
        block_indexer::index_block_with_retries(&pool, &client, &config, 1395, &price)
            .await
            .is_err()
    );
    assert_eq!(state::get_last_indexed_height(&pool).await.unwrap(), 1394);
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM swap_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        before, 1,
        "valid predecessor may persist before genuine infrastructure failure"
    );
    // Metadata recovery makes the same block resolvable; no source amount changes.
    sqlx::query("INSERT INTO assets (contract_address,is_cw20,name,symbol,decimals) VALUES ($1,true,'Recovered','RCVR',6)")
        .bind(unknown).execute(&pool).await.unwrap();
    for _ in 0..2 {
        block_indexer::index_block_with_retries(&pool, &client, &config, 1395, &price)
            .await
            .unwrap();
    }
    assert_eq!(state::get_last_indexed_height(&pool).await.unwrap(), 1395);
    let totals:(i64,i64,i32,i64)=sqlx::query_as(
        "SELECT (SELECT count(*) FROM swap_events),(SELECT count(*) FROM protocol_fee_events),(SELECT trade_count FROM trader_positions WHERE pair_id=$1),(SELECT total_trades FROM traders WHERE address=$2)",
    ).bind(pair).bind(TRADER).fetch_one(&pool).await.unwrap();
    assert_eq!(totals, (2, 2, 2, 2));
    let failed: i64 = sqlx::query_scalar("SELECT count(*) FROM indexer_failed_blocks")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(failed, 0, "successful retry clears failed-block tracking");
}

#[tokio::test]
#[serial]
async fn numeric_boundaries_and_18_decimal_assets_persist_without_rounding() {
    let pool = common::setup_pool().await;
    seed(&pool).await;
    // Large raw amounts are ordinary trades for 18-decimal tokens.
    sqlx::query("UPDATE assets SET decimals=18")
        .execute(&pool)
        .await
        .unwrap();
    let server = MockServer::start().await;
    let txs = [tx(
        "boundary1395",
        vec![
            swap(
                PAIR,
                "10000000000000000000000000",
                "20000000000000000000000000",
                &[("commission_amount", "0")],
            ),
            event(
                "terra1hook",
                "after_swap_burn",
                &[("burn_amount", MAX_RAW), ("burn_token", "uluna")],
            ),
            event(
                PAIR,
                "provide_liquidity",
                &[
                    ("sender", TRADER),
                    ("assets", &format!("uluna {MAX_RAW}, uusd {MAX_RAW}")),
                    ("share", MAX_RAW),
                ],
            ),
            event(
                PAIR,
                "place_limit_order",
                &[
                    ("order_id", "9223372036854775807"),
                    ("owner", TRADER),
                    ("side", "bid"),
                    ("price", MAX_PRICE),
                ],
            ),
            event(
                PAIR,
                "place_limit_order",
                &[
                    ("order_id", "19"),
                    ("owner", TRADER),
                    ("side", "bid"),
                    ("price", "0.000000000000000001"),
                ],
            ),
            event(
                PAIR,
                "limit_order_expired_parked",
                &[
                    ("order_id", "19"),
                    ("maker", TRADER),
                    ("side", "bid"),
                    ("remaining", MAX_RAW),
                ],
            ),
        ],
    )];
    ingest(&pool, &server, &txs).await;
    let swap_row: (String, String, Option<String>) = sqlx::query_as(
        "SELECT offer_amount::text,return_amount::text,commission_amount::text FROM swap_events",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        swap_row,
        (
            "10000000000000000000000000".into(),
            "20000000000000000000000000".into(),
            Some("0".into())
        )
    );
    let hook: String = sqlx::query_scalar("SELECT amount::text FROM hook_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(hook, MAX_RAW);
    let liq: (String, String, String) = sqlx::query_as(
        "SELECT asset_0_amount::text,asset_1_amount::text,lp_amount::text FROM liquidity_events",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(liq, (MAX_RAW.into(), MAX_RAW.into(), MAX_RAW.into()));
    let prices: Vec<BigDecimal> =
        sqlx::query_scalar("SELECT price FROM limit_order_placements ORDER BY order_id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        prices,
        vec![
            "0.000000000000000001".parse::<BigDecimal>().unwrap(),
            MAX_PRICE.parse::<BigDecimal>().unwrap()
        ]
    );
    let remaining: BigDecimal =
        sqlx::query_scalar("SELECT remaining_escrow FROM limit_order_placements WHERE order_id=19")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(remaining, MAX_RAW.parse::<BigDecimal>().unwrap());
    let max_fee_txs = [tx(
        "maxfees1395",
        vec![swap(
            PAIR,
            "10000000000000000000000000",
            "20000000000000000000000000",
            &[("commission_amount", MAX_RAW), ("spread_amount", MAX_RAW)],
        )],
    )];
    ingest(&pool, &server, &max_fee_txs).await;
    ingest(&pool, &server, &max_fee_txs).await;
    let fees: BigDecimal =
        sqlx::query_scalar("SELECT total_fees_paid FROM traders WHERE address=$1")
            .bind(TRADER)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        fees,
        MAX_RAW.parse::<BigDecimal>().unwrap() * BigDecimal::from(2),
        "raw fee accumulation preserves every digit and replay does not add fees again"
    );
}

#[tokio::test]
#[serial]
async fn reorg_guard_and_cursor_replay_preserve_valid_ordinals_and_fee_dedup() {
    let pool = common::setup_pool().await;
    seed(&pool).await;
    let server = MockServer::start().await;
    let txs = [tx(
        "reorg1395",
        vec![
            swap(PAIR, "broken", "1", &[]),
            swap(PAIR, "1000000", "2000000", &[("commission_amount", "30")]),
        ],
    )];
    mount_block(&server, 1395, "canonical1395", &txs).await;
    let client = lcd(&server);
    let config = common::test_config();
    let price = oracle::new_shared_price();
    block_indexer::index_block(&pool, &client, &config, 1395, &price)
        .await
        .unwrap();
    state::set_indexer_checkpoint(&pool, 1395, "orphan1395")
        .await
        .unwrap();
    assert!(matches!(
        block_indexer::verify_checkpoint_unchanged(&client, &pool, 1395).await,
        Err(block_indexer::BlockIndexError::ReorgDetected { .. })
    ));
    // Existing recovery's cursor-only rewind preserves retained source rows.
    state::set_last_indexed_height(&pool, 1394).await.unwrap();
    state::set_state(&pool, state::KEY_LAST_INDEXED_BLOCK_HASH, "")
        .await
        .unwrap();
    block_indexer::index_block(&pool, &client, &config, 1395, &price)
        .await
        .unwrap();
    let stored:(i64,i32,i64)=sqlx::query_as(
        "SELECT (SELECT count(*) FROM swap_events),(SELECT swap_index FROM swap_events),(SELECT count(*) FROM protocol_fee_events)",
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(stored, (1, 1, 1));
    assert_eq!(
        state::get_indexer_checkpoint(&pool).await.unwrap(),
        (1395, Some("canonical1395".into()))
    );

    // Exercise the actual operator cleanup against this isolated test database.
    // Supplying DATABASE_URL and host mode prevents touching a compose/shared DB.
    let cleanup = std::process::Command::new("bash")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../scripts/indexer-reorg-recover.sh"
        ))
        .args(["--height", "1395", "--cleanup-derived", "--apply"])
        .env("DATABASE_URL", &config.database_url)
        .env("POSTGRES_PSQL_MODE", "host")
        .output()
        .expect("run established reorg recovery");
    assert!(
        cleanup.status.success(),
        "reorg recovery failed: {}",
        String::from_utf8_lossy(&cleanup.stderr)
    );
    let cleaned: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM swap_events),(SELECT count(*) FROM protocol_fee_events)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        cleaned,
        (0, 0),
        "orphan source rows and their fees must both roll back"
    );
    cl8y_dex_indexer::indexer::position_tracker::rebuild_all_positions_from_swaps(&pool)
        .await
        .unwrap();
    server.reset().await;
    let canonical = [tx(
        "replacement1395",
        vec![
            swap(PAIR, "broken", "1", &[]),
            swap(PAIR, "3000000", "6000000", &[("commission_amount", "40")]),
        ],
    )];
    mount_block(&server, 1395, "replacementhash1395", &canonical).await;
    for _ in 0..2 {
        block_indexer::index_block(&pool, &client, &config, 1395, &price)
            .await
            .unwrap();
    }
    let reconciled: (String, String, i32) = sqlx::query_as(
        "SELECT (SELECT offer_amount::text FROM swap_events),(SELECT amount_raw::text FROM protocol_fee_events),(SELECT trade_count FROM trader_positions)",
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(reconciled, ("3000000".into(), "40".into(), 1));
}

#[tokio::test]
#[serial]
async fn malformed_columnar_order_keeps_neighbor_amounts_attached_to_original_ids() {
    let pool = common::setup_pool().await;
    seed(&pool).await;
    let server = MockServer::start().await;
    // Contract batch events place action headers before parallel field columns.
    // Invalid middle entries must never shift the last price/fee onto another order.
    let placement = event(
        PAIR,
        "place_limit_order_batch",
        &[
            ("action", "place_limit_order"),
            ("action", "place_limit_order"),
            ("action", "place_limit_order"),
            ("order_id", "101"),
            ("order_id", "102"),
            ("order_id", "103"),
            ("owner", TRADER),
            ("side", "bid"),
            ("price", "1.123456789012345678"),
            ("price", "broken"),
            ("price", "3.123456789012345678"),
            ("maker_fee_amount", "9"),
            ("maker_fee_amount", "broken"),
            ("maker_fee_amount", "11"),
        ],
    );
    let txs = [tx("columnar1395", vec![placement])];
    for _ in 0..2 {
        ingest(&pool, &server, &txs).await;
    }
    let placed: Vec<(i64, BigDecimal, BigDecimal)> = sqlx::query_as(
        "SELECT order_id,price,maker_fee_amount FROM limit_order_placements ORDER BY order_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        placed,
        vec![
            (
                101,
                "1.123456789012345678".parse::<BigDecimal>().unwrap(),
                BigDecimal::from(9)
            ),
            (
                103,
                "3.123456789012345678".parse::<BigDecimal>().unwrap(),
                BigDecimal::from(11)
            ),
        ]
    );
    let fees: Vec<(i64, BigDecimal)> =
        sqlx::query_as("SELECT ordinal,amount_raw FROM protocol_fee_events ORDER BY ordinal")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        fees,
        vec![(101, BigDecimal::from(9)), (103, BigDecimal::from(11))]
    );
}

#[tokio::test]
#[serial]
async fn escrow_migration_preserves_legacy_fractional_evidence_and_accepts_38_digit_integer() {
    let pool = common::setup_pool().await;
    let mut transaction = pool.begin().await.unwrap();
    // A session-local table recreates the old column without altering real event rows.
    // pg_temp shadows the public table only for this transaction's migration replay.
    sqlx::query(
        "CREATE TEMP TABLE limit_order_placements (remaining_escrow NUMERIC(38,18)) ON COMMIT DROP",
    )
    .execute(&mut *transaction)
    .await
    .unwrap();
    sqlx::query("INSERT INTO limit_order_placements VALUES (12.123456789012345678)")
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations/20261005064000_event_remaining_precision.sql"
    ))
    .execute(&mut *transaction)
    .await
    .unwrap();
    sqlx::query("INSERT INTO limit_order_placements (remaining_escrow) VALUES ($1)")
        .bind(MAX_RAW.parse::<BigDecimal>().unwrap())
        .execute(&mut *transaction)
        .await
        .unwrap();
    let amounts: Vec<BigDecimal> = sqlx::query_scalar(
        "SELECT remaining_escrow FROM limit_order_placements ORDER BY remaining_escrow",
    )
    .fetch_all(&mut *transaction)
    .await
    .unwrap();
    assert_eq!(
        amounts,
        vec![
            "12.123456789012345678".parse::<BigDecimal>().unwrap(),
            MAX_RAW.parse::<BigDecimal>().unwrap(),
        ]
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
#[serial]
async fn overflowing_reconstructed_reserves_reject_only_affected_action_and_checkpoint_advances() {
    let pool = common::setup_pool().await;
    let pair = seed(&pool).await;
    sqlx::query(
        "INSERT INTO liquidity_events
         (pair_id,block_height,block_timestamp,tx_hash,provider,event_type,
          asset_0_amount,asset_1_amount,lp_amount,reserve_0,reserve_1)
         VALUES ($1,1394,$2,'prior_reserves1395',$3,'add',0,0,0,$4,$4)",
    )
    .bind(pair)
    .bind(time())
    .bind(TRADER)
    .bind(MAX_RAW.parse::<BigDecimal>().unwrap())
    .execute(&pool)
    .await
    .unwrap();
    let server = MockServer::start().await;
    let txs = [tx(
        "reserves1395",
        vec![
            swap(PAIR, "1000000", "2000000", &[("commission_amount", "30")]),
            swap(
                PAIR,
                "1000000",
                "2000000",
                &[("reserve_0", MAX_RAW), ("reserve_1", MAX_RAW)],
            ),
        ],
    )];
    mount_block(&server, 1395, "reservehash1395", &txs).await;
    block_indexer::index_block(
        &pool,
        &lcd(&server),
        &common::test_config(),
        1395,
        &oracle::new_shared_price(),
    )
    .await
    .unwrap();
    assert_eq!(state::get_last_indexed_height(&pool).await.unwrap(), 1395);
    let reserves: Vec<(Option<BigDecimal>, Option<BigDecimal>)> =
        sqlx::query_as("SELECT reserve_0,reserve_1 FROM swap_events ORDER BY swap_index")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        reserves,
        vec![(
            Some(MAX_RAW.parse::<BigDecimal>().unwrap()),
            Some(MAX_RAW.parse::<BigDecimal>().unwrap())
        )],
        "unrepresentable reconstructed action is quarantined; exact chain reserves persist"
    );
    let trades: i32 =
        sqlx::query_scalar("SELECT trade_count FROM trader_positions WHERE pair_id=$1")
            .bind(pair)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(trades, 1);
    let ordinal: i32 = sqlx::query_scalar("SELECT swap_index FROM swap_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(ordinal, 1);
}

#[tokio::test]
#[serial]
async fn maximum_raw_buy_and_sell_preserve_76_digit_realized_pnl_and_replay_once() {
    let pool = common::setup_pool().await;
    let pair = seed(&pool).await;
    // Mixed decimals keep the sell's human price within NUMERIC(38,18), while
    // raw cost basis and quote exposure legitimately produce a 76-digit loss.
    sqlx::query("UPDATE assets SET decimals=CASE WHEN denom='uluna' THEN 0 ELSE 18 END")
        .execute(&pool)
        .await
        .unwrap();
    let server = MockServer::start().await;
    let txs = [tx(
        "widepnl1395",
        vec![
            swap(PAIR, MAX_RAW, "1", &[]),
            swap(
                PAIR,
                MAX_RAW,
                "1",
                &[("offer_asset", "uusd"), ("ask_asset", "uluna")],
            ),
        ],
    )];
    let price = oracle::new_shared_price();
    *price.write().await = Some("0.02".parse::<BigDecimal>().unwrap());
    let recent_time = Utc::now() - chrono::Duration::seconds(1);
    for _ in 0..2 {
        process_block_txs(
            &pool,
            &lcd(&server),
            &common::test_config(),
            &txs,
            1395,
            recent_time,
            &price,
        )
        .await
        .unwrap();
    }
    let maximum = MAX_RAW.parse::<BigDecimal>().unwrap();
    let expected = BigDecimal::from(1) - &maximum * &maximum;
    let pnl: (BigDecimal, BigDecimal, BigDecimal, BigDecimal, i32) = sqlx::query_as(
        "SELECT tp.realized_pnl,t.total_realized_pnl,t.best_trade_pnl,t.worst_trade_pnl,tp.trade_count
         FROM trader_positions tp JOIN traders t ON t.address=tp.trader_address
         WHERE tp.pair_id=$1 AND tp.trader_address=$2",
    ).bind(pair).bind(TRADER).fetch_one(&pool).await.unwrap();
    assert_eq!(
        pnl,
        (
            expected.clone(),
            expected.clone(),
            expected.clone(),
            expected,
            2
        ),
        "position and trader P&L retain the exact raw loss without overflow or replay addition"
    );
    let ledger: Vec<(i32, BigDecimal, BigDecimal)> = sqlx::query_as(
        "SELECT swap_index,offer_amount,return_amount FROM swap_events ORDER BY swap_index",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        ledger,
        vec![
            (0, maximum.clone(), BigDecimal::from(1)),
            (1, maximum.clone(), BigDecimal::from(1)),
        ]
    );
    let total: BigDecimal = sqlx::query_scalar("SELECT total_volume FROM traders WHERE address=$1")
        .bind(TRADER)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        total,
        &maximum * BigDecimal::from(2),
        "raw lifetime sums must not clip at one event's maximum"
    );
    let candle: (BigDecimal, BigDecimal, i32) = sqlx::query_as(
        "SELECT volume_base,volume_quote,trade_count FROM candles WHERE pair_id=$1 AND interval='1h'",
    ).bind(pair).fetch_one(&pool).await.unwrap();
    let side_volume = &maximum + BigDecimal::from(1);
    assert_eq!(
        candle,
        (side_volume.clone(), side_volume.clone(), 2),
        "raw candle sums preserve MAX+1 in both orientations"
    );
    cl8y_dex_indexer::db::queries::volume::refresh_token_volumes(&pool)
        .await
        .unwrap();
    cl8y_dex_indexer::db::queries::volume::refresh_pair_volumes(&pool)
        .await
        .unwrap();
    cl8y_dex_indexer::db::queries::volume::refresh_global_stats(&pool)
        .await
        .unwrap();
    cl8y_dex_indexer::db::queries::traders::refresh_rolling_volumes(&pool)
        .await
        .unwrap();
    let rolled: (BigDecimal, BigDecimal, BigDecimal) = sqlx::query_as(
        "SELECT (SELECT volume_quote FROM pair_volume_24h WHERE pair_id=$1),
                (SELECT total_volume FROM global_stats_24h WHERE id=1),
                (SELECT volume_24h FROM traders WHERE address=$2)",
    )
    .bind(pair)
    .bind(TRADER)
    .fetch_one(&pool)
    .await
    .unwrap();
    let total = &maximum * BigDecimal::from(2);
    assert_eq!(rolled, (side_volume, total.clone(), total));
    let token_raw: Vec<BigDecimal> = sqlx::query_scalar(
        "SELECT volume FROM token_volume_stats WHERE \"window\"='24h' ORDER BY asset_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(token_raw, vec![maximum.clone(), maximum]);
}
