//! RFC 4180-style CSV helpers for trader history exports (GitLab #163).

use super::pairs::{LimitCancellationResponse, LimitFillResponse, TradeResponse};

fn csv_escape_cell(s: &str) -> String {
    let must_quote = s.contains(',')
        || s.contains('"')
        || s.contains('\n')
        || s.contains('\r')
        || s.starts_with(' ')
        || s.ends_with(' ');
    if must_quote {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn join_row(cells: &[String]) -> String {
    cells
        .iter()
        .map(|c| csv_escape_cell(c))
        .collect::<Vec<_>>()
        .join(",")
}

/// `text/csv` body for [`TradeResponse`] rows (UTF-8, header row).
pub fn trader_swaps_csv(rows: &[TradeResponse]) -> String {
    let mut out = String::from(
        "id,pair_address,block_height,block_timestamp,tx_hash,sender,offer_asset,ask_asset,offer_amount,return_amount,price,pool_return_amount,book_return_amount,pool_leg_volume,book_leg_volume,limit_book_offer_consumed,effective_fee_bps,commission_amount,spread_amount\n",
    );
    for r in rows {
        let row = join_row(&[
            r.id.to_string(),
            r.pair_address.clone(),
            r.block_height.to_string(),
            r.block_timestamp.clone(),
            r.tx_hash.clone(),
            r.sender.clone(),
            r.offer_asset.clone(),
            r.ask_asset.clone(),
            r.offer_amount.clone(),
            r.return_amount.clone(),
            r.price.clone(),
            r.pool_return_amount.clone().unwrap_or_default(),
            r.book_return_amount.clone().unwrap_or_default(),
            r.pool_leg_volume.clone().unwrap_or_default(),
            r.book_leg_volume.clone().unwrap_or_default(),
            r.limit_book_offer_consumed.clone().unwrap_or_default(),
            r.effective_fee_bps
                .map(|x| x.to_string())
                .unwrap_or_default(),
            r.commission_amount.clone().unwrap_or_default(),
            r.spread_amount.clone().unwrap_or_default(),
        ]);
        out.push_str(&row);
        out.push('\n');
    }
    out
}

pub fn trader_limit_fills_csv(rows: &[LimitFillResponse]) -> String {
    let mut out = String::from(
        "id,pair_address,swap_event_id,block_height,block_timestamp,tx_hash,order_id,side,maker,price,token0_amount,token1_amount,commission_amount\n",
    );
    for r in rows {
        let row = join_row(&[
            r.id.to_string(),
            r.pair_address.clone(),
            r.swap_event_id.map(|x| x.to_string()).unwrap_or_default(),
            r.block_height.to_string(),
            r.block_timestamp.clone(),
            r.tx_hash.clone(),
            r.order_id.to_string(),
            r.side.clone(),
            r.maker.clone(),
            r.price.clone(),
            r.token0_amount.clone(),
            r.token1_amount.clone(),
            r.commission_amount.clone(),
        ]);
        out.push_str(&row);
        out.push('\n');
    }
    out
}

pub fn trader_limit_cancellations_csv(rows: &[LimitCancellationResponse]) -> String {
    let mut out =
        String::from("id,pair_address,block_height,block_timestamp,tx_hash,order_id,owner\n");
    for r in rows {
        let row = join_row(&[
            r.id.to_string(),
            r.pair_address.clone(),
            r.block_height.to_string(),
            r.block_timestamp.clone(),
            r.tx_hash.clone(),
            r.order_id.to_string(),
            r.owner.clone().unwrap_or_default(),
        ]);
        out.push_str(&row);
        out.push('\n');
    }
    out
}
