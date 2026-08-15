use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, StdError, Timestamp, Uint128};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct RateLimitConfig {
    pub max_amount_per_window: Uint128,
    pub window_seconds: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    NotifyDeposit {
        depositor: String,
        denom: String,
        amount: Uint128,
    },
    Receive(Cw20ReceiveMsg),
    SetDenomMapping {
        denom: String,
        cw20_addr: String,
    },
    RemoveDenomMapping {
        denom: String,
    },
    SetRateLimit {
        denom: String,
        config: RateLimitConfig,
    },
    RemoveRateLimit {
        denom: String,
    },
    ProposeGovernanceTransfer {
        new_governance: String,
    },
    AcceptGovernanceTransfer {},
    CancelGovernanceTransfer {},
    SetPaused {
        paused: bool,
    },
    SetFeeBps {
        fee_bps: u16,
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    Unwrap { recipient: Option<String> },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(DenomMappingResponse)]
    DenomMapping { denom: String },
    #[returns(AllDenomMappingsResponse)]
    AllDenomMappings {},
    #[returns(RateLimitResponse)]
    RateLimit { denom: String },
    #[returns(PendingGovernanceResponse)]
    PendingGovernance {},
}

/// Wrap-mapper `Config` as the DEX router sees it (GitLab #523 / #516).
///
/// Post ustr-cmm#9 migrate the on-chain type is `fee_wrap_bps` / `fee_unwrap_bps`
/// and **drops** `fee_bps`. Pre-migrate columbus-5 still returns `{ fee_bps }` only.
/// All three fields are optional so `#[cw_serde]` (`deny_unknown_fields`) can
/// deserialize either shape. Resolve fees with [`wrap_mapper_fee_pair`] (W13).
#[cw_serde]
pub struct ConfigResponse {
    pub governance: Addr,
    pub treasury: Addr,
    pub paused: bool,
    #[serde(default)]
    pub fee_bps: Option<u16>,
    #[serde(default)]
    pub fee_wrap_bps: Option<u16>,
    #[serde(default)]
    pub fee_unwrap_bps: Option<u16>,
}

impl ConfigResponse {
    /// Unwrap skim for router `minimum_receive` (**R3**).
    pub fn unwrap_fee_bps(&self) -> Result<u16, StdError> {
        wrap_mapper_fee_pair(self.fee_bps, self.fee_wrap_bps, self.fee_unwrap_bps).map(|(_, u)| u)
    }
}

/// Fail closed on partial/invalid split fees. Transitional `{ fee_bps }` only when
/// both split fields are absent (pre-migrate). Never treat missing as 0% (W13).
pub fn wrap_mapper_fee_pair(
    fee_bps: Option<u16>,
    fee_wrap_bps: Option<u16>,
    fee_unwrap_bps: Option<u16>,
) -> Result<(u16, u16), StdError> {
    match (fee_wrap_bps, fee_unwrap_bps) {
        (Some(wrap), Some(unwrap)) => Ok((wrap, unwrap)),
        (None, None) => match fee_bps {
            Some(legacy) => Ok((legacy, legacy)),
            None => Err(StdError::generic_err(
                "wrap-mapper Config missing fee_unwrap_bps / fee_bps",
            )),
        },
        _ => Err(StdError::generic_err(
            "wrap-mapper Config has partial split fees",
        )),
    }
}

#[cw_serde]
pub struct PendingGovernanceResponse {
    pub new_governance: Option<Addr>,
    pub execute_after: Option<Timestamp>,
}

#[cw_serde]
pub struct DenomMappingResponse {
    pub denom: String,
    pub cw20_addr: Addr,
}

#[cw_serde]
pub struct DenomMappingEntry {
    pub denom: String,
    pub cw20_addr: Addr,
}

#[cw_serde]
pub struct AllDenomMappingsResponse {
    pub mappings: Vec<DenomMappingEntry>,
}

#[cw_serde]
pub struct RateLimitResponse {
    pub config: Option<RateLimitConfig>,
    pub current_window_start: Option<Timestamp>,
    pub amount_used: Uint128,
}

/// Treasury execute messages needed by the router and wrap-mapper integration.
#[cw_serde]
pub enum TreasuryExecuteMsg {
    WrapDeposit {},
    InstantWithdraw {
        recipient: String,
        denom: String,
        amount: Uint128,
    },
    SetDenomWrapper {
        denom: String,
        wrapper: String,
    },
    RemoveDenomWrapper {
        denom: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::from_json;

    fn cfg(json: &str) -> ConfigResponse {
        from_json(json.as_bytes()).expect("ConfigResponse")
    }

    #[test]
    fn config_legacy_fee_bps_maps_to_unwrap() {
        let c =
            cfg(r#"{"governance":"terra1g","treasury":"terra1t","paused":false,"fee_bps":200}"#);
        assert_eq!(c.fee_bps, Some(200));
        assert_eq!(c.fee_wrap_bps, None);
        assert_eq!(c.fee_unwrap_bps, None);
        assert_eq!(c.unwrap_fee_bps().unwrap(), 200);
        assert_eq!(
            wrap_mapper_fee_pair(c.fee_bps, c.fee_wrap_bps, c.fee_unwrap_bps).unwrap(),
            (200, 200)
        );
    }

    #[test]
    fn config_split_fees_no_fee_bps() {
        let c = cfg(
            r#"{"governance":"terra1g","treasury":"terra1t","paused":false,"fee_wrap_bps":200,"fee_unwrap_bps":51}"#,
        );
        assert_eq!(c.fee_bps, None);
        assert_eq!(c.unwrap_fee_bps().unwrap(), 51);
        assert_eq!(
            wrap_mapper_fee_pair(c.fee_bps, c.fee_wrap_bps, c.fee_unwrap_bps).unwrap(),
            (200, 51)
        );
    }

    #[test]
    fn config_split_fees_win_over_legacy() {
        let c = cfg(
            r#"{"governance":"terra1g","treasury":"terra1t","paused":false,"fee_bps":200,"fee_wrap_bps":200,"fee_unwrap_bps":51}"#,
        );
        assert_eq!(c.unwrap_fee_bps().unwrap(), 51);
    }

    #[test]
    fn config_partial_split_fails_closed() {
        let wrap_only = cfg(
            r#"{"governance":"terra1g","treasury":"terra1t","paused":false,"fee_wrap_bps":200}"#,
        );
        let err = wrap_only.unwrap_fee_bps().unwrap_err();
        assert!(err.to_string().contains("partial split"), "got {err}");

        let unwrap_only = cfg(
            r#"{"governance":"terra1g","treasury":"terra1t","paused":false,"fee_unwrap_bps":51}"#,
        );
        assert!(unwrap_only
            .unwrap_fee_bps()
            .unwrap_err()
            .to_string()
            .contains("partial split"));
    }

    #[test]
    fn config_missing_all_fees_fails_closed() {
        let c = cfg(r#"{"governance":"terra1g","treasury":"terra1t","paused":false}"#);
        let err = c.unwrap_fee_bps().unwrap_err();
        assert!(
            err.to_string().contains("missing fee_unwrap_bps"),
            "got {err}"
        );
    }
}
