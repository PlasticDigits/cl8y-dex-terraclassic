//! LCD-backed limit order book walks (on-chain FIFO list via `order_book_head` + `limit_order`).

use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use super::limit_book_price::{compare_positive_decimal_strings, validate_limit_book_price, DecimalCmp};
use crate::lcd::{LcdClient, LcdError};

/// Default page size for `GET .../limit-book`.
pub const LIMIT_BOOK_PAGE_DEFAULT: i64 = 50;
/// Max orders per `GET .../limit-book` request (each order may require one LCD `limit_order` query).
pub const LIMIT_BOOK_PAGE_MAX: i64 = 100;
/// Max LCD smart queries per deep limit-book page (head + cursor + one per order). Documented H7 budget.
pub const LIMIT_BOOK_LCD_QUERY_BUDGET: usize = 101;
/// Max target prices per `insert-hints` request (pair `max_batch_rungs` hard cap).
pub const LIMIT_BOOK_INSERT_HINTS_MAX_PRICES: usize = 100;

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

/// LocalTerra / wasmd often return HTTP 500 for missing contract keys instead of `{ "data": null }`.
fn lcd_query_missing_key(err: &LcdError) -> bool {
    match err {
        LcdError::AllEndpointsFailed(msg) | LcdError::ContractQueryRejected(msg) => {
            msg.contains("not found")
        }
        _ => false,
    }
}

async fn fetch_limit_order(
    lcd: &LcdClient,
    pair_addr: &str,
    order_id: u64,
) -> Result<Option<ChainLimitOrderRow>, LimitBookLcdError> {
    match lcd
        .query_contract(
            pair_addr,
            &json!({ "limit_order": { "order_id": order_id } }),
        )
        .await
    {
        Ok(row) => Ok(row),
        Err(e) if lcd_query_missing_key(&e) => Ok(None),
        Err(e) => Err(LimitBookLcdError::Lcd(e)),
    }
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
    let mut lcd_queries = 0usize;
    let mut bump_lcd = || -> Result<(), LimitBookLcdError> {
        lcd_queries += 1;
        if lcd_queries > LIMIT_BOOK_LCD_QUERY_BUDGET {
            return Err(LimitBookLcdError::BadRequest(
                "limit-book LCD query budget exceeded for this page".to_string(),
            ));
        }
        Ok(())
    };

    let mut current: Option<u64> = match after_order_id {
        None => {
            bump_lcd()?;
            lcd.query_contract(
                pair_addr,
                &json!({ "order_book_head": { "side": side_label } }),
            )
            .await?
        }
        Some(prev_id) => {
            bump_lcd()?;
            let row_opt = fetch_limit_order(lcd, pair_addr, prev_id).await?;
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
        bump_lcd()?;
        let row_opt = fetch_limit_order(lcd, pair_addr, oid).await?;
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

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LimitBookInsertHintItem {
    pub price: String,
    pub predecessor_order_id: Option<u64>,
    pub resolved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LimitBookInsertHintsResult {
    pub hints: Vec<LimitBookInsertHintItem>,
    pub budget_exhausted: bool,
}

#[derive(Debug, Clone)]
struct PendingInsertHint {
    request_index: usize,
    price: String,
}

fn resolve_hint_on_order(
    side_label: &'static str,
    target_price: &str,
    order_price: &str,
    prev_order_id: Option<u64>,
) -> Option<LimitBookInsertHintItem> {
    let cmp = compare_positive_decimal_strings(target_price, order_price)?;
    match side_label {
        "bid" if cmp == DecimalCmp::Gt => Some(LimitBookInsertHintItem {
            price: target_price.to_string(),
            predecessor_order_id: prev_order_id,
            resolved: true,
            reason: if prev_order_id.is_none() {
                Some("head".to_string())
            } else {
                None
            },
        }),
        "ask" if cmp == DecimalCmp::Lt => Some(LimitBookInsertHintItem {
            price: target_price.to_string(),
            predecessor_order_id: prev_order_id,
            resolved: true,
            reason: if prev_order_id.is_none() {
                Some("head".to_string())
            } else {
                None
            },
        }),
        _ => None,
    }
}

fn apply_pending_tail_hints(
    results: &mut [Option<LimitBookInsertHintItem>],
    pending: &[PendingInsertHint],
    prev_order_id: Option<u64>,
    has_more: bool,
) {
    for p in pending {
        results[p.request_index] = Some(if has_more {
            LimitBookInsertHintItem {
                price: p.price.clone(),
                predecessor_order_id: None,
                resolved: false,
                reason: Some("pagination_gap".to_string()),
            }
        } else {
            LimitBookInsertHintItem {
                price: p.price.clone(),
                predecessor_order_id: prev_order_id,
                resolved: true,
                reason: None,
            }
        });
    }
}

/// Batch-resolve insert predecessors for `target_prices` with one head→tail LCD walk (GitLab #267).
pub async fn resolve_limit_insert_hints(
    lcd: &LcdClient,
    pair_addr: &str,
    side_label: &'static str,
    target_prices: &[String],
) -> Result<LimitBookInsertHintsResult, LimitBookLcdError> {
    if target_prices.is_empty() {
        return Err(LimitBookLcdError::BadRequest(
            "prices must contain at least one entry".to_string(),
        ));
    }
    if target_prices.len() > LIMIT_BOOK_INSERT_HINTS_MAX_PRICES {
        return Err(LimitBookLcdError::BadRequest(format!(
            "prices list exceeds max {} entries",
            LIMIT_BOOK_INSERT_HINTS_MAX_PRICES
        )));
    }

    let mut validated: Vec<String> = Vec::with_capacity(target_prices.len());
    for raw in target_prices {
        validated.push(
            validate_limit_book_price(raw)
                .map_err(LimitBookLcdError::BadRequest)?,
        );
    }

    let mut results: Vec<Option<LimitBookInsertHintItem>> =
        vec![None; validated.len()];
    let mut pending: Vec<PendingInsertHint> = validated
        .iter()
        .enumerate()
        .map(|(i, price)| PendingInsertHint {
            request_index: i,
            price: price.clone(),
        })
        .collect();

    let mut lcd_queries = 0usize;
    let mut bump_lcd = || -> Result<(), LimitBookLcdError> {
        lcd_queries += 1;
        if lcd_queries > LIMIT_BOOK_LCD_QUERY_BUDGET {
            return Err(LimitBookLcdError::BadRequest(
                "limit-book LCD query budget exceeded".to_string(),
            ));
        }
        Ok(())
    };

    bump_lcd()?;
    let mut current: Option<u64> = lcd
        .query_contract(
            pair_addr,
            &json!({ "order_book_head": { "side": side_label } }),
        )
        .await?;

    let mut prev_order_id: Option<u64> = None;
    let mut budget_exhausted = false;

    if current.is_none() {
        for p in &pending {
            results[p.request_index] = Some(LimitBookInsertHintItem {
                price: p.price.clone(),
                predecessor_order_id: None,
                resolved: true,
                reason: Some("head".to_string()),
            });
        }
        return Ok(LimitBookInsertHintsResult {
            hints: results.into_iter().flatten().collect(),
            budget_exhausted: false,
        });
    }

    while !pending.is_empty() {
        let Some(oid) = current else {
            break;
        };
        if bump_lcd().is_err() {
            budget_exhausted = true;
            break;
        }
        let row_opt = fetch_limit_order(lcd, pair_addr, oid).await?;
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

        let order_price = row.price.clone();
        let mut still_pending = Vec::new();
        for p in pending {
            if let Some(resolved) = resolve_hint_on_order(
                side_label,
                &p.price,
                &order_price,
                prev_order_id,
            ) {
                results[p.request_index] = Some(resolved);
            } else {
                still_pending.push(p);
            }
        }
        pending = still_pending;
        prev_order_id = Some(row.order_id);
        current = row.next;
    }

    let has_more = current.is_some() || budget_exhausted;
    apply_pending_tail_hints(&mut results, &pending, prev_order_id, has_more);

    let hints: Vec<LimitBookInsertHintItem> = results
        .into_iter()
        .zip(validated.iter())
        .map(|(opt, price)| {
            opt.unwrap_or_else(|| LimitBookInsertHintItem {
                price: price.clone(),
                predecessor_order_id: None,
                resolved: false,
                reason: Some("pagination_gap".to_string()),
            })
        })
        .collect();

    Ok(LimitBookInsertHintsResult {
        hints,
        budget_exhausted,
    })
}

fn order_in_price_window(
    side_label: &'static str,
    order_price: &str,
    price_from: &str,
    price_to: &str,
) -> Result<Option<bool>, LimitBookLcdError> {
    let cmp_from = compare_positive_decimal_strings(order_price, price_from)
        .ok_or_else(|| LimitBookLcdError::BadRequest("invalid price window bounds".to_string()))?;
    let cmp_to = compare_positive_decimal_strings(order_price, price_to)
        .ok_or_else(|| LimitBookLcdError::BadRequest("invalid price window bounds".to_string()))?;
    Ok(match side_label {
        // Bids: head = highest price; band is [price_to, price_from] inclusive (price_from >= price_to).
        "bid" => {
            let above_band = cmp_from == DecimalCmp::Gt;
            let below_band = cmp_to == DecimalCmp::Lt;
            if above_band {
                Some(false)
            } else if below_band {
                None
            } else {
                Some(true)
            }
        }
        // Asks: head = lowest price; band is [price_from, price_to] inclusive (price_to >= price_from).
        "ask" => {
            let below_band = cmp_from == DecimalCmp::Lt;
            let above_band = cmp_to == DecimalCmp::Gt;
            if below_band {
                Some(false)
            } else if above_band {
                None
            } else {
                Some(true)
            }
        }
        _ => None,
    })
}

/// Walk the on-chain book and return the contiguous slice covering `[price_from, price_to]` (GitLab #267).
///
/// GitLab #270: on a page-full stop this peeks the one successor node to decide `has_more` (see the
/// post-loop comment). A side effect is that a broken/wrong-side immediate successor surfaces its
/// corrupt-book 400 one page earlier than the full-book route would — the same integrity checks the
/// walk loop already applies to every node; a healthy book never triggers it.
pub async fn fetch_limit_book_price_window(
    lcd: &LcdClient,
    pair_addr: &str,
    side_label: &'static str,
    price_from: &str,
    price_to: &str,
    page_limit: i64,
    after_order_id: Option<u64>,
) -> Result<(Vec<LimitBookOrderItem>, bool, Option<u64>), LimitBookLcdError> {
    let price_from = validate_limit_book_price(price_from).map_err(LimitBookLcdError::BadRequest)?;
    let price_to = validate_limit_book_price(price_to).map_err(LimitBookLcdError::BadRequest)?;
    let band_ok = match side_label {
        "bid" => compare_positive_decimal_strings(&price_from, &price_to) != Some(DecimalCmp::Lt),
        "ask" => compare_positive_decimal_strings(&price_to, &price_from) != Some(DecimalCmp::Lt),
        _ => false,
    };
    if !band_ok {
        return Err(LimitBookLcdError::BadRequest(
            "price_from/price_to do not form a valid band for this side".to_string(),
        ));
    }

    let mut lcd_queries = 0usize;
    let mut bump_lcd = || -> Result<(), LimitBookLcdError> {
        lcd_queries += 1;
        if lcd_queries > LIMIT_BOOK_LCD_QUERY_BUDGET {
            return Err(LimitBookLcdError::BadRequest(
                "limit-book LCD query budget exceeded for this page".to_string(),
            ));
        }
        Ok(())
    };

    let mut current: Option<u64> = match after_order_id {
        None => {
            bump_lcd()?;
            lcd.query_contract(
                pair_addr,
                &json!({ "order_book_head": { "side": side_label } }),
            )
            .await?
        }
        Some(prev_id) => {
            bump_lcd()?;
            let row_opt = fetch_limit_order(lcd, pair_addr, prev_id).await?;
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
    let mut past_band = false;

    while orders.len() < cap && !past_band {
        let Some(oid) = current else {
            break;
        };
        bump_lcd()?;
        let row_opt = fetch_limit_order(lcd, pair_addr, oid).await?;
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

        let next = row.next;
        match order_in_price_window(side_label, &row.price, &price_from, &price_to)? {
            Some(true) => orders.push(row_to_item(row)),
            Some(false) => {}
            None => past_band = true,
        }
        if past_band {
            break;
        }
        current = next;
    }

    // GitLab #270: `has_more` for a price window means "another page may return more IN-BAND rows",
    // NOT "the FIFO chain continues outside the band". The book is price-ordered, so once the walk
    // leaves the band every later node is out-of-band; `current` may still point at such a node.
    //
    //  - past_band  -> the in-band region is exhausted -> has_more = false (no extra LCD query).
    //  - chain end  -> has_more = false.
    //  - page full  -> the successor is unclassified; peek its band membership and only count an
    //                  in-band successor as "more". The peek costs one LCD query, so it is gated on
    //                  the per-page budget: if the budget is already spent we conservatively keep
    //                  has_more = true (a bounded over-report — the next page returns the remaining
    //                  in-band rows or one empty past-band page, then false). The common past_band
    //                  case adds zero queries, so the budget ceiling (101) is preserved.
    let has_more = if past_band {
        false
    } else {
        match current {
            None => false,
            Some(oid) => {
                if bump_lcd().is_err() {
                    true
                } else {
                    let row_opt = fetch_limit_order(lcd, pair_addr, oid).await?;
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
                    matches!(
                        order_in_price_window(side_label, &row.price, &price_from, &price_to)?,
                        Some(true)
                    )
                }
            }
        }
    };
    let next_after_order_id = if has_more {
        orders.last().map(|o| o.order_id)
    } else {
        None
    };

    Ok((orders, has_more, next_after_order_id))
}
