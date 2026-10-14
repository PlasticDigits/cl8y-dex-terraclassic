//! Explicit mode controls are exact budgets, never optimizer suggestions.
use super::route_solver::HybridHopJson;

pub(super) fn declared_split(hybrid: Option<&HybridHopJson>, offer: u128) -> Option<(u128, u32)> {
    let Some(h) = hybrid else {
        return Some((0, 8));
    };
    let pool = h.pool_input.parse::<u128>().ok()?;
    let book = h.book_input.parse::<u128>().ok()?;
    if pool.checked_add(book)? != offer
        || h.book_start_hint.is_some()
        || h.max_maker_fills == 0
        || h.max_maker_fills > crate::hybrid_limits::MAX_MAKER_FILLS_HARD_CAP
    {
        return None;
    }
    Some((book, h.max_maker_fills))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_split_budget_and_supported_book_controls_are_required() {
        let mut h = HybridHopJson {
            pool_input: "6".into(),
            book_input: "4".into(),
            max_maker_fills: 8,
            book_start_hint: None,
        };
        assert_eq!(declared_split(Some(&h), 10), Some((4, 8)));
        assert_eq!(declared_split(None, 10), Some((0, 8)));
        assert!(declared_split(Some(&h), 11).is_none());
        h.book_start_hint = Some(1);
        assert!(declared_split(Some(&h), 10).is_none());
        h.book_start_hint = None;
        h.max_maker_fills = 0;
        assert!(declared_split(Some(&h), 10).is_none());
        h.max_maker_fills = 8;
        h.pool_input = u128::MAX.to_string();
        assert!(declared_split(Some(&h), 10).is_none());
    }
}

/// A direct native mapping is a zero-pair operation with its own reviewed plan.
/// It still requires fresh independent reference/config observations and positive
/// wallet/base floors; identity equality cannot manufacture a free swap.
pub(super) fn wrapper_only(
    s: &super::Snapshot,
    normalized: &super::route_solver::SolveRouteParams,
    n: &super::NativeContext,
    spread: u128,
) -> Option<serde_json::Value> {
    use super::{net_floor, now_ms};
    use serde_json::json;
    if n.wrap == n.unwrap || normalized.token_in != normalized.token_out {
        return None;
    }
    let gross = if let Some(raw) = n.request.amount_in.as_deref() {
        raw.parse::<u128>().ok()?
    } else {
        let target = n.request.amount_out.as_deref()?.parse::<u128>().ok()?;
        if n.wrap {
            n.wrapper.inverse_wrap(target)?
        } else {
            n.wrapper.inverse_unwrap(&n.request.token_out, target)?
        }
    };
    let (route_amount, output) = if n.wrap {
        let (_, minted) = n.wrapper.wrap_input(&n.request.token_in, gross, now_ms())?;
        (minted, minted)
    } else {
        let (_, net, _) = n
            .wrapper
            .unwrap_output_direct(&n.request.token_out, gross, now_ms())?;
        (gross, net)
    };
    let target = n
        .request
        .amount_out
        .as_deref()
        .map(str::parse::<u128>)
        .transpose()
        .ok()?
        .unwrap_or(0);
    let floor = net_floor(s, output, spread)?.max(target).max(1);
    if floor > output {
        return None;
    }
    let minimum_base = if n.unwrap {
        let needed = n.wrapper.inverse_unwrap(&n.request.token_out, floor)?;
        n.wrapper
            .unwrap_output_direct(&n.request.token_out, needed, now_ms())?
            .2
    } else {
        floor
    };
    let id = *s.ids.get(&normalized.token_in)?;
    let (fair, reference_at) = s
        .references
        .fair_output(id, id, gross, chrono::Utc::now())?;
    if super::BigInt::from(fair.abs_diff(output)) * super::BigInt::from(100u8)
        >= super::BigInt::from(fair.max(output)) * super::BigInt::from(99u8)
    {
        return None;
    }
    let observed = n.wrapper.observed_at_ms.min(reference_at);
    let expires = (observed + 20_000).min(s.config_expiry_ms);
    if expires <= now_ms() {
        return None;
    }
    let spread_text = format!("0.{spread:018}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string();
    let floor_protocol = n.wrapper.direct_floor_protocol(n.wrap);
    let supported = floor_protocol.is_some();
    let wrapper = json!({"input_token":n.request.token_in,"output_token":n.request.token_out,"input_amount":gross.to_string(),"route_input_amount":route_amount.to_string(),"route_output_amount":route_amount.to_string(),"minimum_receive_base":minimum_base.to_string(),"wrap_input":n.wrap,"unwrap_output":n.unwrap,"treasury":n.wrapper.treasury,"mapper":n.wrapper.mapper,"floor_protocol_version":floor_protocol});
    let mut metadata = json!({"version":1,"generation":s.generation,"input_amount":gross.to_string(),"max_spread":spread_text,"trader":n.request.trader,"sender":n.request.sender,"observed_at_ms":observed,"expires_at_ms":expires,"policy":"wrapper_only","hops":[],"minimum_receive":floor.to_string(),"wrapper":wrapper,"execution_supported":supported});
    if s.quote_relative_protection {
        metadata["protection_policy"] = json!("quote_relative_v1");
    }
    let mut body = json!({
        "token_in":n.request.token_in,"token_out":n.request.token_out,"hops":[],"router_operations":[],
        "intermediate_tokens":[n.request.token_in,n.request.token_out],"quote_kind":"indexer_wrapper_db",
        "solver_version":"local_v1","tax_kind":"none","buy_tax_bps":0,"sell_tax_bps":0,"router_hops_tax":false,
        "estimated_amount_out":output.to_string(),"estimated_amount_out_net":output.to_string(),
        "spot_amount_out":fair.to_string(),"slippage_percent":format!("{:.4}",fair.abs_diff(output) as f64 / fair.max(output) as f64 * 100.0),
        "required_amount_in":n.request.amount_out.as_ref().map(|_|gross.to_string()),"target_amount_out":n.request.amount_out,
        "sizing_mode":n.request.amount_out.as_ref().map(|_|"local_pool_exact_output"),"paths_considered":0,"lcd_hybrid_queries":0,"db_hybrid_queries":0,
        "snapshot_age_ms":now_ms()-observed,"optimality_scope":"Exact direct native mapping; no pair route",
        "execution_metadata":metadata,"execution_blocker":if supported {serde_json::Value::Null}else{json!("Direct wrapping cannot enforce the reviewed receive floor")}
    });
    body["economic_attribution"] = super::attribution::wrapper_only(gross, fair, output, &body);
    body["order_economics"] = super::order_economics::wrapper_only(gross, &body);
    Some(body)
}
