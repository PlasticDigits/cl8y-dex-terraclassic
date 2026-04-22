//! LCD-backed limit order book walks (on-chain FIFO list via `order_book_head` + `limit_order`).

use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use crate::lcd::{LcdClient, LcdError};

/// Default page size for `GET .../limit-book`.
pub const LIMIT_BOOK_PAGE_DEFAULT: i64 = 50;
/// Max orders per `GET .../limit-book` request (each order may require one LCD `limit_order` query).
pub const LIMIT_BOOK_PAGE_MAX: i64 = 100;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ChainLimitOrderRow {
    pub order_id: u64,
    pub owner: String,
    pub side: serde_json::Value,
    pub price: String,
    pub remaining: String,
    #[serde(default)]
    pub expires_at: Option<u64>,
    #[allow(dead_code)]
    pub prev: Option<u64>,
    pub next: Option<u64>,
}

pub fn chain_side_label(v: &serde_json::Value) -> String {
    v.as_str()
        .map(std::string::ToString::to_string)
        .or_else(|| {
            v.as_object()
                .and_then(|m| m.keys().next().map(std::string::ToString::to_string))
        })
        .unwrap_or_else(|| "unknown".to_string())
}

#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct LimitBookOrderItem {
    pub order_id: u64,
    pub owner: String,
    pub side: String,
    pub price: String,
    pub remaining: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
}

#[derive(Debug, thiserror::Error)]
pub enum LimitBookLcdError {
    #[error(transparent)]
    Lcd(#[from] LcdError),
    #[error("{0}")]
    BadRequest(String),
}

fn side_matches_book(side_label: &str, row: &ChainLimitOrderRow) -> bool {
    chain_side_label(&row.side).eq_ignore_ascii_case(side_label)
}

fn row_to_item(row: ChainLimitOrderRow) -> LimitBookOrderItem {
    LimitBookOrderItem {
        order_id: row.order_id,
        owner: row.owner,
        side: chain_side_label(&row.side),
        price: row.price,
        remaining: row.remaining,
        expires_at: row.expires_at,
    }
}

/// Walk up to `page_limit` resting orders starting from book head, or continuing after `after_order_id`.
///
/// LCD calls per successful page: `order_book_head` when `after_order_id` is `None`; otherwise one
/// `limit_order` lookup for the cursor; plus one `limit_order` query per returned order.
pub async fn fetch_limit_book_page(
    lcd: &LcdClient,
    pair_addr: &str,
    side_label: &'static str,
    page_limit: i64,
    after_order_id: Option<u64>,
) -> Result<(Vec<LimitBookOrderItem>, bool, Option<u64>), LimitBookLcdError> {
    let mut current: Option<u64> = match after_order_id {
        None => {
            lcd.query_contract(
                pair_addr,
                &json!({ "order_book_head": { "side": side_label } }),
            )
            .await?
        }
        Some(prev_id) => {
            let row_opt: Option<ChainLimitOrderRow> = lcd
                .query_contract(
                    pair_addr,
                    &json!({ "limit_order": { "order_id": prev_id } }),
                )
                .await?;
            let Some(row) = row_opt else {
                return Err(LimitBookLcdError::BadRequest(format!(
                    "Unknown after_order_id: {prev_id}"
                )));
            };
            if !side_matches_book(side_label, &row) {
                return Err(LimitBookLcdError::BadRequest(
                    "after_order_id is not on the requested book side".to_string(),
                ));
            }
            row.next
        }
    };

    let mut orders = Vec::new();
    let cap = page_limit as usize;

    while orders.len() < cap {
        let Some(oid) = current else {
            break;
        };
        let row_opt: Option<ChainLimitOrderRow> = lcd
            .query_contract(pair_addr, &json!({ "limit_order": { "order_id": oid } }))
            .await?;
        let Some(row) = row_opt else {
            return Err(LimitBookLcdError::BadRequest(format!(
                "Broken book link: limit_order {oid} missing"
            )));
        };
        if !side_matches_book(side_label, &row) {
            return Err(LimitBookLcdError::BadRequest(
                "Order side does not match requested book side".to_string(),
            ));
        }
        current = row.next;
        orders.push(row_to_item(row));
    }

    let has_more = current.is_some();
    let next_after_order_id = if has_more {
        orders.last().map(|o| o.order_id)
    } else {
        None
    };

    Ok((orders, has_more, next_after_order_id))
}
