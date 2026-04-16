//! GitLab #82 — integration: hybrid wasm attrs parse to `swap_events` hybrid columns.

mod common;

use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::db::queries::swap_events::{self, SwapEventRow};
use cl8y_dex_indexer::indexer::parser::parse_swaps;
use cl8y_dex_indexer::lcd::{Attribute, Event, TxLog, TxResponse};
use serial_test::serial;
use std::str::FromStr;

fn wasm_tx_hybrid_swap(pair_addr: &str) -> TxResponse {
    let attrs = vec![
        ("contract_address", pair_addr),
        ("action", "swap"),
        ("sender", "terra1traderxyz"),
        ("offer_amount", "100"),
        ("return_amount", "95"),
        ("offer_asset", "uluna"),
        ("ask_asset", "terra1ustctoken"),
        ("pool_return_amount", "40"),
        ("book_return_amount", "55"),
        ("limit_book_offer_consumed", "60"),
        ("effective_fee_bps", "30"),
    ];
    TxResponse {
        height: "1".into(),
        txhash: "hybrid_swap_cols_integration_tx".into(),
        logs: Some(vec![TxLog {
            events: vec![Event {
                event_type: "wasm".into(),
                attributes: attrs
                    .into_iter()
                    .map(|(k, v)| Attribute {
                        key: k.to_string(),
                        value: v.to_string(),
                    })
                    .collect(),
            }],
        }]),
        timestamp: None,
        events: None,
    }
}

#[serial]
#[tokio::test]
async fn hybrid_swap_attrs_round_trip_to_swap_events_columns() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    let tx = wasm_tx_hybrid_swap(&seed.pair_address);
    let swaps = parse_swaps(&tx);
    assert_eq!(swaps.len(), 1);
    let s = &swaps[0];

    assert_eq!(s.effective_fee_bps, Some(30));
    assert_eq!(
        s.pool_return_amount.as_ref().map(|b| b.to_string()),
        Some("40".into())
    );
    assert_eq!(
        s.book_return_amount.as_ref().map(|b| b.to_string()),
        Some("55".into())
    );
    assert_eq!(
        s.limit_book_offer_consumed.as_ref().map(|b| b.to_string()),
        Some("60".into())
    );

    let block_time = Utc::now();
    let height: i64 = 99_001;
    let tx_hash = tx.txhash.as_str();
    let price = &s.return_amount / &s.offer_amount;

    let id = swap_events::insert_swap(
        &pool,
        seed.pair_id,
        height,
        block_time,
        tx_hash,
        &s.sender,
        s.receiver.as_deref(),
        seed.asset_0_id,
        seed.asset_1_id,
        &s.offer_amount,
        &s.return_amount,
        s.spread_amount.as_ref(),
        s.commission_amount.as_ref(),
        s.effective_fee_bps,
        &price,
        None,
        s.pool_return_amount.as_ref(),
        s.book_return_amount.as_ref(),
        s.limit_book_offer_consumed.as_ref(),
    )
    .await
    .expect("insert_swap");
    assert!(id.is_some(), "expected insert to succeed");

    let row: SwapEventRow = sqlx::query_as(
        "SELECT id, pair_id, block_height, block_timestamp, tx_hash, sender, receiver,
                offer_asset_id, ask_asset_id, offer_amount, return_amount,
                spread_amount, commission_amount, effective_fee_bps, price, volume_usd,
                pool_return_amount, book_return_amount, limit_book_offer_consumed
         FROM swap_events WHERE tx_hash = $1 AND pair_id = $2",
    )
    .bind(tx_hash)
    .bind(seed.pair_id)
    .fetch_one(&pool)
    .await
    .expect("row");

    assert_eq!(row.effective_fee_bps, Some(30));
    assert_eq!(
        row.pool_return_amount
            .as_ref()
            .map(|b| b.to_string())
            .as_deref(),
        Some("40")
    );
    assert_eq!(
        row.book_return_amount
            .as_ref()
            .map(|b| b.to_string())
            .as_deref(),
        Some("55")
    );
    assert_eq!(
        row.limit_book_offer_consumed
            .as_ref()
            .map(|b| b.to_string())
            .as_deref(),
        Some("60")
    );
    assert_eq!(row.offer_amount, BigDecimal::from_str("100").unwrap());
    assert_eq!(row.return_amount, BigDecimal::from_str("95").unwrap());
}
