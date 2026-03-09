use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdError, StdResult, Uint128};

/// TerraSwap-compatible asset identifier. Supports both CW20 tokens and native tokens
/// in the type system for wire compatibility, but our DEX currently only accepts CW20 tokens.
#[cw_serde]
pub enum AssetInfo {
    Token { contract_addr: String },
    NativeToken { denom: String },
}

impl AssetInfo {
    pub fn assert_is_token(&self) -> StdResult<&str> {
        match self {
            AssetInfo::Token { contract_addr } => Ok(contract_addr.as_str()),
            AssetInfo::NativeToken { .. } => Err(StdError::generic_err(
                "Native tokens are not supported; use CW20 wrapped tokens instead",
            )),
        }
    }

    pub fn is_native_token(&self) -> bool {
        matches!(self, AssetInfo::NativeToken { .. })
    }

    pub fn canonical_key(&self) -> String {
        match self {
            AssetInfo::Token { contract_addr } => format!("token:{}", contract_addr),
            AssetInfo::NativeToken { denom } => format!("native:{}", denom),
        }
    }

    pub fn equal(&self, other: &AssetInfo) -> bool {
        match (self, other) {
            (
                AssetInfo::Token { contract_addr: a },
                AssetInfo::Token { contract_addr: b },
            ) => a == b,
            (
                AssetInfo::NativeToken { denom: a },
                AssetInfo::NativeToken { denom: b },
            ) => a == b,
            _ => false,
        }
    }
}

impl std::fmt::Display for AssetInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetInfo::Token { contract_addr } => write!(f, "{}", contract_addr),
            AssetInfo::NativeToken { denom } => write!(f, "{}", denom),
        }
    }
}

/// TerraSwap-compatible asset: an AssetInfo paired with an amount.
#[cw_serde]
pub struct Asset {
    pub info: AssetInfo,
    pub amount: Uint128,
}

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.info, self.amount)
    }
}

/// Return `asset_infos` in canonical (sorted) order for deduplication.
pub fn canonical_order(asset_infos: [AssetInfo; 2]) -> [AssetInfo; 2] {
    let key_0 = asset_infos[0].canonical_key();
    let key_1 = asset_infos[1].canonical_key();
    if key_0 <= key_1 {
        asset_infos
    } else {
        let [a, b] = asset_infos;
        [b, a]
    }
}

/// Build a deterministic storage key from a pair of AssetInfos.
pub fn pair_key(asset_infos: &[AssetInfo; 2]) -> String {
    let mut keys = [
        asset_infos[0].canonical_key(),
        asset_infos[1].canonical_key(),
    ];
    keys.sort();
    format!("{}|{}", keys[0], keys[1])
}

/// TerraSwap-compatible pair info returned in queries.
#[cw_serde]
pub struct PairInfo {
    pub asset_infos: [AssetInfo; 2],
    pub contract_addr: Addr,
    pub liquidity_token: Addr,
}

#[cw_serde]
pub struct FeeConfig {
    pub fee_bps: u16,
    pub treasury: Addr,
}
