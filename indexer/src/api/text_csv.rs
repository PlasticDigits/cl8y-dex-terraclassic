//! RFC 4180-style CSV helpers for trader history exports (GitLab #163).

use super::pairs::{LimitCancellationResponse, LimitFillResponse, TradeResponse};

/// Spreadsheet formula prefixes (`=`, `+`, `-`, `@`) are neutralized with a leading `'`
/// so Excel/LibreOffice/Sheets do not interpret the cell as a formula (SEC-F12 / #432).
fn csv_escape_cell(s: &str) -> String {
    let neutralized = if s.starts_with(['=', '+', '-', '@']) {
        format!("'{s}")
    } else {
        s.to_string()
    };
    let must_quote = neutralized.contains(',')
        || neutralized.contains('"')
        || neutralized.contains('\n')
        || neutralized.contains('\r')
        || neutralized.starts_with(' ')
        || neutralized.ends_with(' ');
    if must_quote {
        format!("\"{}\"", neutralized.replace('"', "\"\""))
    } else {
        neutralized
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
        "id,pair_address,block_height,block_timestamp,tx_hash,sender,offer_asset,ask_asset,offer_amount,return_amount,price,pool_return_amount,book_return_amount,pool_leg_volume,book_leg_volume,limit_book_offer_consumed,effective_fee_bps,commission_amount,spread_amount,offer_decimals,ask_decimals\n",
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
            r.offer_decimals.map(|d| d.to_string()).unwrap_or_default(),
            r.ask_decimals.map(|d| d.to_string()).unwrap_or_default(),
        ]);
        out.push_str(&row);
        out.push('\n');
    }
    out
}

pub fn trader_limit_fills_csv(rows: &[LimitFillResponse]) -> String {
    let mut out = String::from(
        "id,pair_address,swap_event_id,block_height,block_timestamp,tx_hash,order_id,side,maker,price,token0_amount,token1_amount,commission_amount,token0_decimals,token1_decimals\n",
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
            r.token0_decimals.map(|d| d.to_string()).unwrap_or_default(),
            r.token1_decimals.map(|d| d.to_string()).unwrap_or_default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_escape_cell_neutralizes_bare_formula_prefix() {
        let escaped = csv_escape_cell("=1+1");
        assert_eq!(escaped, "'=1+1");
    }

    #[test]
    fn csv_escape_cell_neutralizes_formula_prefixes() {
        for prefix in ['=', '+', '-', '@'] {
            let input = format!("{prefix}HYPERLINK(\"http://evil\")");
            let escaped = csv_escape_cell(&input);
            assert!(
                !escaped.starts_with(prefix),
                "cell must not start with formula char {prefix}: {escaped}"
            );
            assert!(
                escaped.starts_with('\'') || escaped.contains(&format!("'{prefix}")),
                "expected single-quote formula neutralizer in cell: {escaped}"
            );
        }
    }

    #[test]
    fn csv_escape_cell_neutralizes_formula_prefix_with_commas() {
        let input = "=1+1,extra";
        let escaped = csv_escape_cell(input);
        assert!(!escaped.starts_with('='));
        assert!(escaped.starts_with('"'));
        assert!(escaped.contains("'=1+1,extra"));
    }

    #[test]
    fn trader_swaps_csv_neutralizes_formula_in_offer_asset() {
        let row = TradeResponse {
            id: 1,
            pair_address: "terra1pair".to_string(),
            block_height: 100,
            block_timestamp: "2026-01-01T00:00:00Z".to_string(),
            tx_hash: "ABC123".to_string(),
            sender: "terra1sender".to_string(),
            offer_asset: "=HYPERLINK(\"http://evil\")".to_string(),
            ask_asset: "uluna".to_string(),
            offer_amount: "1000".to_string(),
            return_amount: "900".to_string(),
            offer_decimals: Some(6),
            ask_decimals: Some(6),
            price: "0.9".to_string(),
            price_usd: None,
            pool_return_amount: None,
            book_return_amount: None,
            limit_book_offer_consumed: None,
            effective_fee_bps: None,
            commission_amount: None,
            spread_amount: None,
            pool_leg_volume: None,
            book_leg_volume: None,
        };
        let csv = trader_swaps_csv(&[row]);
        let data_line = csv.lines().nth(1).expect("data row");
        assert!(
            !data_line.contains(",=HYPERLINK"),
            "offer_asset must not appear as raw formula: {data_line}"
        );
        assert!(
            data_line.contains("'=HYPERLINK"),
            "offer_asset must be formula-neutralized: {data_line}"
        );
    }
}
