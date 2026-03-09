# Classic TerraSwap Upstream Source Code Reference

Source: https://github.com/terraswap/classic-terraswap (branch: `main`)

The package crate is `classic_terraswap` (note underscore), located at `packages/classic_terraswap/src/`.

---

## Package: `classic_terraswap` — lib.rs

```rust
pub mod asset;
pub mod factory;
pub mod pair;
pub mod querier;
pub mod router;
pub mod token;
pub mod util;

#[cfg(not(target_arch = "wasm32"))]
pub mod mock_querier;

#[cfg(test)]
mod testing;
```

---

## Package: `classic_terraswap::asset` — asset.rs

```rust
use lazy_regex::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::querier::{query_balance, query_native_decimals, query_token_balance, query_token_info};
use classic_bindings::{TerraMsg, TerraQuerier, TerraQuery};
use cosmwasm_std::{
    to_binary, Addr, Api, BankMsg, CanonicalAddr, Coin, CosmosMsg, Decimal, MessageInfo,
    QuerierWrapper, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

pub static IBC_REX: Lazy<Regex> = lazy_regex!("^ibc/[A-F0-9]{64}$");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Asset {
    pub info: AssetInfo,
    pub amount: Uint128,
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.amount, self.info)
    }
}

static DECIMAL_FRACTION: Uint128 = Uint128::new(1_000_000_000_000_000_000u128);

impl Asset {
    pub fn is_native_token(&self) -> bool {
        self.info.is_native_token()
    }

    pub fn compute_tax(&self, querier: &QuerierWrapper<TerraQuery>) -> StdResult<Uint128> {
        let amount = self.amount;
        if let AssetInfo::NativeToken { denom } = &self.info {
            if self.info.is_ibc_token() {
                return Ok(Uint128::zero());
            }

            let terra_querier = TerraQuerier::new(querier);
            let tax_rate: Decimal = (terra_querier.query_tax_rate()?).rate;
            let tax_cap: Uint128 = (terra_querier.query_tax_cap(denom.to_string())?).cap;
            Ok(std::cmp::min(
                amount.checked_sub(amount.multiply_ratio(
                    DECIMAL_FRACTION,
                    DECIMAL_FRACTION * tax_rate + DECIMAL_FRACTION,
                ))?,
                tax_cap,
            ))
        } else {
            Ok(Uint128::zero())
        }
    }

    pub fn deduct_tax(&self, querier: &QuerierWrapper<TerraQuery>) -> StdResult<Coin> {
        let amount = self.amount;
        if let AssetInfo::NativeToken { denom } = &self.info {
            Ok(Coin {
                denom: denom.to_string(),
                amount: amount.checked_sub(self.compute_tax(querier)?)?,
            })
        } else {
            Err(StdError::generic_err("cannot deduct tax from token asset"))
        }
    }

    pub fn into_msg(
        self,
        querier: &QuerierWrapper<TerraQuery>,
        recipient: Addr,
    ) -> StdResult<CosmosMsg<TerraMsg>> {
        let amount = self.amount;

        match &self.info {
            AssetInfo::Token { contract_addr } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: recipient.to_string(),
                    amount,
                })?,
                funds: vec![],
            })),
            AssetInfo::NativeToken { .. } => Ok(CosmosMsg::Bank(BankMsg::Send {
                to_address: recipient.to_string(),
                amount: vec![self.deduct_tax(querier)?],
            })),
        }
    }

    pub fn into_submsg(
        self,
        querier: &QuerierWrapper<TerraQuery>,
        recipient: Addr,
    ) -> StdResult<SubMsg<TerraMsg>> {
        Ok(SubMsg::new(self.into_msg(querier, recipient)?))
    }

    pub fn assert_sent_native_token_balance(&self, message_info: &MessageInfo) -> StdResult<()> {
        if let AssetInfo::NativeToken { denom } = &self.info {
            match message_info.funds.iter().find(|x| x.denom == *denom) {
                Some(coin) => {
                    if self.amount == coin.amount {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                }
                None => {
                    if self.amount.is_zero() {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                }
            }
        } else {
            Ok(())
        }
    }

    pub fn to_raw(&self, api: &dyn Api) -> StdResult<AssetRaw> {
        Ok(AssetRaw {
            info: match &self.info {
                AssetInfo::NativeToken { denom } => AssetInfoRaw::NativeToken {
                    denom: denom.to_string(),
                },
                AssetInfo::Token { contract_addr } => AssetInfoRaw::Token {
                    contract_addr: api.addr_canonicalize(contract_addr.as_str())?,
                },
            },
            amount: self.amount,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetInfo {
    Token { contract_addr: String },
    NativeToken { denom: String },
}

impl fmt::Display for AssetInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AssetInfo::NativeToken { denom } => write!(f, "{denom}"),
            AssetInfo::Token { contract_addr } => write!(f, "{contract_addr}"),
        }
    }
}

impl AssetInfo {
    pub fn to_raw(&self, api: &dyn Api) -> StdResult<AssetInfoRaw> {
        match self {
            AssetInfo::NativeToken { denom } => Ok(AssetInfoRaw::NativeToken {
                denom: denom.to_string(),
            }),
            AssetInfo::Token { contract_addr } => Ok(AssetInfoRaw::Token {
                contract_addr: api.addr_canonicalize(contract_addr.as_str())?,
            }),
        }
    }

    pub fn is_native_token(&self) -> bool {
        match self {
            AssetInfo::NativeToken { .. } => true,
            AssetInfo::Token { .. } => false,
        }
    }

    pub fn is_ibc_token(&self) -> bool {
        match self {
            AssetInfo::NativeToken { denom } => {
                if IBC_REX.is_match(denom) {
                    return true;
                }
                false
            }
            AssetInfo::Token { .. } => false,
        }
    }

    pub fn query_pool(
        &self,
        querier: &QuerierWrapper<TerraQuery>,
        api: &dyn Api,
        pool_addr: Addr,
    ) -> StdResult<Uint128> {
        match self {
            AssetInfo::Token { contract_addr, .. } => query_token_balance(
                querier,
                api.addr_validate(contract_addr.as_str())?,
                pool_addr,
            ),
            AssetInfo::NativeToken { denom, .. } => {
                query_balance(querier, pool_addr, denom.to_string())
            }
        }
    }

    pub fn equal(&self, asset: &AssetInfo) -> bool {
        match self {
            AssetInfo::Token { contract_addr, .. } => {
                let self_contract_addr = contract_addr;
                match asset {
                    AssetInfo::Token { contract_addr, .. } => self_contract_addr == contract_addr,
                    AssetInfo::NativeToken { .. } => false,
                }
            }
            AssetInfo::NativeToken { denom, .. } => {
                let self_denom = denom;
                match asset {
                    AssetInfo::Token { .. } => false,
                    AssetInfo::NativeToken { denom, .. } => self_denom == denom,
                }
            }
        }
    }

    pub fn query_decimals(
        &self,
        account_addr: Addr,
        querier: &QuerierWrapper<TerraQuery>,
    ) -> StdResult<u8> {
        match self {
            AssetInfo::NativeToken { denom } => {
                query_native_decimals(querier, account_addr, denom.to_string())
            }
            AssetInfo::Token { contract_addr } => {
                let token_info = query_token_info(querier, Addr::unchecked(contract_addr))?;
                Ok(token_info.decimals)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct AssetRaw {
    pub info: AssetInfoRaw,
    pub amount: Uint128,
}

impl AssetRaw {
    pub fn to_normal(&self, api: &dyn Api) -> StdResult<Asset> {
        Ok(Asset {
            info: match &self.info {
                AssetInfoRaw::NativeToken { denom } => AssetInfo::NativeToken {
                    denom: denom.to_string(),
                },
                AssetInfoRaw::Token { contract_addr } => AssetInfo::Token {
                    contract_addr: api.addr_humanize(contract_addr)?.to_string(),
                },
            },
            amount: self.amount,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum AssetInfoRaw {
    Token { contract_addr: CanonicalAddr },
    NativeToken { denom: String },
}

impl AssetInfoRaw {
    pub fn to_normal(&self, api: &dyn Api) -> StdResult<AssetInfo> {
        match self {
            AssetInfoRaw::NativeToken { denom } => Ok(AssetInfo::NativeToken {
                denom: denom.to_string(),
            }),
            AssetInfoRaw::Token { contract_addr } => Ok(AssetInfo::Token {
                contract_addr: api.addr_humanize(contract_addr)?.to_string(),
            }),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            AssetInfoRaw::NativeToken { denom } => denom.as_bytes(),
            AssetInfoRaw::Token { contract_addr } => contract_addr.as_slice(),
        }
    }

    pub fn equal(&self, asset: &AssetInfoRaw) -> bool {
        match self {
            AssetInfoRaw::Token { contract_addr, .. } => {
                let self_contract_addr = contract_addr;
                match asset {
                    AssetInfoRaw::Token { contract_addr, .. } => {
                        self_contract_addr == contract_addr
                    }
                    AssetInfoRaw::NativeToken { .. } => false,
                }
            }
            AssetInfoRaw::NativeToken { denom, .. } => {
                let self_denom = denom;
                match asset {
                    AssetInfoRaw::Token { .. } => false,
                    AssetInfoRaw::NativeToken { denom, .. } => self_denom == denom,
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PairInfo {
    pub asset_infos: [AssetInfo; 2],
    pub contract_addr: String,
    pub liquidity_token: String,
    pub asset_decimals: [u8; 2],
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PairInfoRaw {
    pub asset_infos: [AssetInfoRaw; 2],
    pub contract_addr: CanonicalAddr,
    pub liquidity_token: CanonicalAddr,
    pub asset_decimals: [u8; 2],
}

impl PairInfoRaw {
    pub fn to_normal(&self, api: &dyn Api) -> StdResult<PairInfo> {
        Ok(PairInfo {
            liquidity_token: api.addr_humanize(&self.liquidity_token)?.to_string(),
            contract_addr: api.addr_humanize(&self.contract_addr)?.to_string(),
            asset_infos: [
                self.asset_infos[0].to_normal(api)?,
                self.asset_infos[1].to_normal(api)?,
            ],
            asset_decimals: self.asset_decimals,
        })
    }

    pub fn query_pools(
        &self,
        querier: &QuerierWrapper<TerraQuery>,
        api: &dyn Api,
        contract_addr: Addr,
    ) -> StdResult<[Asset; 2]> {
        let info_0: AssetInfo = self.asset_infos[0].to_normal(api)?;
        let info_1: AssetInfo = self.asset_infos[1].to_normal(api)?;
        Ok([
            Asset {
                amount: info_0.query_pool(querier, api, contract_addr.clone())?,
                info: info_0,
            },
            Asset {
                amount: info_1.query_pool(querier, api, contract_addr)?,
                info: info_1,
            },
        ])
    }
}
```

---

## Package: `classic_terraswap::pair` — pair.rs

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::asset::{Asset, AssetInfo};

use cosmwasm_std::{Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub asset_infos: [AssetInfo; 2],
    pub token_code_id: u64,
    pub asset_decimals: [u8; 2],
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    ProvideLiquidity {
        assets: [Asset; 2],
        receiver: Option<String>,
        deadline: Option<u64>,
        slippage_tolerance: Option<Decimal>,
    },
    Swap {
        offer_asset: Asset,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
        deadline: Option<u64>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    Swap {
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
        deadline: Option<u64>,
    },
    WithdrawLiquidity {
        min_assets: Option<[Asset; 2]>,
        deadline: Option<u64>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Pair {},
    Pool {},
    Simulation { offer_asset: Asset },
    ReverseSimulation { ask_asset: Asset },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PoolResponse {
    pub assets: [Asset; 2],
    pub total_share: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct SimulationResponse {
    pub return_amount: Uint128,
    pub spread_amount: Uint128,
    pub commission_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ReverseSimulationResponse {
    pub offer_amount: Uint128,
    pub spread_amount: Uint128,
    pub commission_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MigrateMsg {}
```

---

## Package: `classic_terraswap::factory` — factory.rs

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::asset::{Asset, AssetInfo, PairInfo};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub pair_code_id: u64,
    pub token_code_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateConfig {
        owner: Option<String>,
        token_code_id: Option<u64>,
        pair_code_id: Option<u64>,
    },
    CreatePair {
        assets: [Asset; 2],
    },
    AddNativeTokenDecimals {
        denom: String,
        decimals: u8,
    },
    MigratePair {
        contract: String,
        code_id: Option<u64>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Pair {
        asset_infos: [AssetInfo; 2],
    },
    Pairs {
        start_after: Option<[AssetInfo; 2]>,
        limit: Option<u32>,
    },
    NativeTokenDecimals {
        denom: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub pair_code_id: u64,
    pub token_code_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PairsResponse {
    pub pairs: Vec<PairInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct NativeTokenDecimalsResponse {
    pub decimals: u8,
}
```

---

## Package: `classic_terraswap::router` — router.rs

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;

use crate::asset::AssetInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub terraswap_factory: String,
    pub loop_factory: String,
    pub astroport_factory: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SwapOperation {
    NativeSwap {
        offer_denom: String,
        ask_denom: String,
    },
    TerraSwap {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
    Loop {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
    Astroport {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
}

impl SwapOperation {
    pub fn get_target_asset_info(&self) -> AssetInfo {
        match self {
            SwapOperation::NativeSwap { ask_denom, .. } => AssetInfo::NativeToken {
                denom: ask_denom.clone(),
            },
            SwapOperation::TerraSwap { ask_asset_info, .. }
            | SwapOperation::Loop { ask_asset_info, .. }
            | SwapOperation::Astroport { ask_asset_info, .. } => ask_asset_info.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    ExecuteSwapOperations {
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        to: Option<String>,
        deadline: Option<u64>,
    },
    ExecuteSwapOperation {
        operation: SwapOperation,
        to: Option<String>,
        deadline: Option<u64>,
    },
    AssertMinimumReceive {
        asset_info: AssetInfo,
        prev_balance: Uint128,
        minimum_receive: Uint128,
        receiver: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    ExecuteSwapOperations {
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        to: Option<String>,
        deadline: Option<u64>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    SimulateSwapOperations {
        offer_amount: Uint128,
        operations: Vec<SwapOperation>,
    },
    ReverseSimulateSwapOperations {
        ask_amount: Uint128,
        operations: Vec<SwapOperation>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ConfigResponse {
    pub terraswap_factory: String,
    pub loop_factory: String,
    pub astroport_factory: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct SimulateSwapOperationsResponse {
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MigrateMsg {}
```

---

## Package: `classic_terraswap::token` — token.rs

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{StdError, StdResult, Uint128};
use cw20::{Cw20Coin, MinterResponse};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Vec<Cw20Coin>,
    pub mint: Option<MinterResponse>,
}

impl InstantiateMsg {
    pub fn get_cap(&self) -> Option<Uint128> {
        self.mint.as_ref().and_then(|v| v.cap)
    }

    pub fn validate(&self) -> StdResult<()> {
        if !is_valid_name(&self.name) {
            return Err(StdError::generic_err(
                "Name is not in the expected format (3-50 UTF-8 bytes)",
            ));
        }
        if !is_valid_symbol(&self.symbol) {
            return Err(StdError::generic_err(
                "Ticker symbol is not in expected format [a-zA-Z\\-]{3,12}",
            ));
        }
        if self.decimals > 18 {
            return Err(StdError::generic_err("Decimals must not exceed 18"));
        }
        Ok(())
    }
}

fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 50 {
        return false;
    }
    true
}

fn is_valid_symbol(symbol: &str) -> bool {
    let bytes = symbol.as_bytes();
    if bytes.len() < 3 || bytes.len() > 12 {
        return false;
    }
    for byte in bytes.iter() {
        if (*byte != 45) && (*byte < 65 || *byte > 90) && (*byte < 97 || *byte > 122) {
            return false;
        }
    }
    true
}
```

---

## Package: `classic_terraswap::querier` — querier.rs

```rust
use crate::asset::{Asset, AssetInfo, PairInfo};
use crate::factory::{NativeTokenDecimalsResponse, QueryMsg as FactoryQueryMsg};
use crate::pair::{QueryMsg as PairQueryMsg, ReverseSimulationResponse, SimulationResponse};

use classic_bindings::TerraQuery;
use cosmwasm_std::{
    to_binary, Addr, AllBalanceResponse, BalanceResponse, BankQuery, Coin, QuerierWrapper,
    QueryRequest, StdResult, Uint128, WasmQuery,
};

use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

pub fn query_balance(
    querier: &QuerierWrapper<TerraQuery>,
    account_addr: Addr,
    denom: String,
) -> StdResult<Uint128> {
    let balance: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: account_addr.to_string(),
        denom,
    }))?;
    Ok(balance.amount.amount)
}

pub fn query_all_balances(
    querier: &QuerierWrapper<TerraQuery>,
    account_addr: Addr,
) -> StdResult<Vec<Coin>> {
    let all_balances: AllBalanceResponse =
        querier.query(&QueryRequest::Bank(BankQuery::AllBalances {
            address: account_addr.to_string(),
        }))?;
    Ok(all_balances.amount)
}

pub fn query_token_balance(
    querier: &QuerierWrapper<TerraQuery>,
    contract_addr: Addr,
    account_addr: Addr,
) -> StdResult<Uint128> {
    let res: Cw20BalanceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: account_addr.to_string(),
        })?,
    }))?;
    Ok(res.balance)
}

pub fn query_token_info(
    querier: &QuerierWrapper<TerraQuery>,
    contract_addr: Addr,
) -> StdResult<TokenInfoResponse> {
    let token_info: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;
    Ok(token_info)
}

pub fn query_native_decimals(
    querier: &QuerierWrapper<TerraQuery>,
    factory_contract: Addr,
    denom: String,
) -> StdResult<u8> {
    let res: NativeTokenDecimalsResponse =
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: factory_contract.to_string(),
            msg: to_binary(&FactoryQueryMsg::NativeTokenDecimals { denom })?,
        }))?;
    Ok(res.decimals)
}

pub fn query_pair_info(
    querier: &QuerierWrapper<TerraQuery>,
    factory_contract: Addr,
    asset_infos: &[AssetInfo; 2],
) -> StdResult<PairInfo> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: factory_contract.to_string(),
        msg: to_binary(&FactoryQueryMsg::Pair {
            asset_infos: asset_infos.clone(),
        })?,
    }))
}

pub fn simulate(
    querier: &QuerierWrapper<TerraQuery>,
    pair_contract: Addr,
    offer_asset: &Asset,
) -> StdResult<SimulationResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_contract.to_string(),
        msg: to_binary(&PairQueryMsg::Simulation {
            offer_asset: offer_asset.clone(),
        })?,
    }))
}

pub fn reverse_simulate(
    querier: &QuerierWrapper<TerraQuery>,
    pair_contract: Addr,
    ask_asset: &Asset,
) -> StdResult<ReverseSimulationResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_contract.to_string(),
        msg: to_binary(&PairQueryMsg::ReverseSimulation {
            ask_asset: ask_asset.clone(),
        })?,
    }))
}

pub fn query_pair_info_from_pair(
    querier: &QuerierWrapper<TerraQuery>,
    pair_contract: Addr,
) -> StdResult<PairInfo> {
    let pair_info: PairInfo = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_contract.to_string(),
        msg: to_binary(&PairQueryMsg::Pair {})?,
    }))?;
    Ok(pair_info)
}
```

---

## Package: `classic_terraswap::util` — util.rs

```rust
use classic_bindings::TerraQuery;
use cosmwasm_std::{DepsMut, StdError, StdResult};
use cw2::{get_contract_version, set_contract_version};

pub fn assert_deadline(blocktime: u64, deadline: Option<u64>) -> StdResult<()> {
    if let Some(deadline) = deadline {
        if blocktime >= deadline {
            return Err(StdError::generic_err("Expired deadline"));
        }
    }
    Ok(())
}

pub fn migrate_version(
    deps: DepsMut<TerraQuery>,
    target_contract_version: &str,
    name: &str,
    version: &str,
) -> StdResult<()> {
    let prev_version = get_contract_version(deps.as_ref().storage)?;
    if prev_version.contract != name {
        return Err(StdError::generic_err("invalid contract"));
    }

    if prev_version.version != target_contract_version {
        return Err(StdError::generic_err(format!(
            "invalid contract version. target {}, but source is {}",
            target_contract_version, prev_version.version
        )));
    }

    set_contract_version(deps.storage, name, version)?;

    Ok(())
}
```

---

## Contract: `terraswap_factory` — state.rs

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use classic_terraswap::asset::{AssetInfoRaw, AssetRaw, PairInfo, PairInfoRaw};
use cosmwasm_std::{Addr, Api, CanonicalAddr, Order, StdResult, Storage};
use cw_storage_plus::{Bound, Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub pair_code_id: u64,
    pub token_code_id: u64,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct TmpPairInfo {
    pub pair_key: Vec<u8>,
    pub assets: [AssetRaw; 2],
    pub asset_decimals: [u8; 2],
    pub sender: Addr,
}

pub const TMP_PAIR_INFO: Item<TmpPairInfo> = Item::new("tmp_pair_info");
pub const PAIRS: Map<&[u8], PairInfoRaw> = Map::new("pair_info");

pub fn pair_key(asset_infos: &[AssetInfoRaw; 2]) -> Vec<u8> {
    let mut asset_infos = asset_infos.to_vec();
    asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    [asset_infos[0].as_bytes(), asset_infos[1].as_bytes()].concat()
}

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_pairs(
    storage: &dyn Storage,
    api: &dyn Api,
    start_after: Option<[AssetInfoRaw; 2]>,
    limit: Option<u32>,
) -> StdResult<Vec<PairInfo>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after).map(Bound::ExclusiveRaw);

    PAIRS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            v.to_normal(api)
        })
        .collect::<StdResult<Vec<PairInfo>>>()
}

fn calc_range_start(start_after: Option<[AssetInfoRaw; 2]>) -> Option<Vec<u8>> {
    start_after.map(|asset_infos| {
        let mut asset_infos = asset_infos.to_vec();
        asset_infos.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

        let mut v = [asset_infos[0].as_bytes(), asset_infos[1].as_bytes()]
            .concat()
            .as_slice()
            .to_vec();
        v.push(1);
        v
    })
}

pub const ALLOW_NATIVE_TOKENS: Map<&[u8], u8> = Map::new("allow_native_token");
pub fn add_allow_native_token(
    storage: &mut dyn Storage,
    denom: String,
    decimals: u8,
) -> StdResult<()> {
    ALLOW_NATIVE_TOKENS.save(storage, denom.as_bytes(), &decimals)
}
```

---

## Contract: `terraswap_factory` — contract.rs

```rust
use classic_terraswap::querier::{query_balance, query_pair_info_from_pair};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    ReplyOn, Response, StdError, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::response::MsgInstantiateContractResponse;
use crate::state::{
    add_allow_native_token, pair_key, read_pairs, Config, TmpPairInfo, ALLOW_NATIVE_TOKENS, CONFIG,
    PAIRS, TMP_PAIR_INFO,
};

use classic_bindings::{TerraMsg, TerraQuery};

use classic_terraswap::asset::{Asset, AssetInfo, AssetInfoRaw, PairInfo, PairInfoRaw};
use classic_terraswap::factory::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, NativeTokenDecimalsResponse,
    PairsResponse, QueryMsg,
};
use classic_terraswap::pair::{
    ExecuteMsg as PairExecuteMsg, InstantiateMsg as PairInstantiateMsg,
    MigrateMsg as PairMigrateMsg,
};
use classic_terraswap::util::migrate_version;
use protobuf::Message;

const CONTRACT_NAME: &str = "crates.io:terraswap-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const CREATE_PAIR_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<TerraQuery>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response<TerraMsg>> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_canonicalize(info.sender.as_str())?,
        token_code_id: msg.token_code_id,
        pair_code_id: msg.pair_code_id,
    };

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<TerraQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response<TerraMsg>> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            token_code_id,
            pair_code_id,
        } => execute_update_config(deps, env, info, owner, token_code_id, pair_code_id),
        ExecuteMsg::CreatePair { assets } => execute_create_pair(deps, env, info, assets),
        ExecuteMsg::AddNativeTokenDecimals { denom, decimals } => {
            execute_add_native_token_decimals(deps, env, info, denom, decimals)
        }
        ExecuteMsg::MigratePair { contract, code_id } => {
            execute_migrate_pair(deps, env, info, contract, code_id)
        }
    }
}

// ... (full execute handlers, reply, query, migrate as shown above)
```

---

## Contract: `terraswap_pair` — state.rs

```rust
use classic_terraswap::asset::PairInfoRaw;
use cw_storage_plus::Item;

pub const PAIR_INFO: Item<PairInfoRaw> = Item::new("pair_info");
```

---

## Contract: `terraswap_pair` — error.rs

```rust
use cosmwasm_std::{ConversionOverflowError, OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    ConversionOverflowError(#[from] ConversionOverflowError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Max spread assertion")]
    MaxSpreadAssertion {},

    #[error("Asset mismatch")]
    AssetMismatch {},

    #[error("Min amount assertion ({min_asset} > {asset})")]
    MinAmountAssertion { min_asset: String, asset: String },

    #[error("Max slippage assertion")]
    MaxSlippageAssertion {},

    #[error("More initial liquidity needed ({min_lp_token} > {given_lp})")]
    MinimumLiquidityAmountError {
        min_lp_token: String,
        given_lp: String,
    },
}
```

---

## Contract: `terraswap_pair` — contract.rs (key constants & functions)

```rust
const CONTRACT_NAME: &str = "crates.io:terraswap-pair";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const INSTANTIATE_REPLY_ID: u64 = 1;

/// Commission rate == 0.3%
const COMMISSION_RATE: u64 = 3;
const MINIMUM_LIQUIDITY_AMOUNT: u128 = 1_000;
```

Key functions:
- `instantiate` — saves PairInfoRaw with empty liquidity_token, creates LP token via SubMsg
- `execute` — dispatches Receive, ProvideLiquidity, Swap
- `receive_cw20` — handles Cw20HookMsg::Swap and Cw20HookMsg::WithdrawLiquidity
- `reply` — captures LP token address from instantiate reply
- `provide_liquidity` — calculates share using sqrt(deposit0 * deposit1) for initial, min ratio for subsequent
- `withdraw_liquidity` — burns LP, returns pro-rata assets
- `swap` — constant product AMM with 0.3% commission
- `compute_swap(offer_pool, ask_pool, offer_amount) -> (return_amount, spread_amount, commission_amount)`:
  - `return_amount = (ask_pool * offer_amount) / (offer_pool + offer_amount)`
  - `commission = return_amount * 0.003` (rounded up)
  - `return_amount = return_amount - commission`
- `compute_offer_amount` — reverse calculation
- `assert_max_spread` — checks belief_price and max_spread
- `assert_minimum_assets` — checks min withdrawal amounts

---

## Contract: `terraswap_router` — state.rs

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::CanonicalAddr;
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub terraswap_factory: CanonicalAddr,
    pub loop_factory: CanonicalAddr,
    pub astroport_factory: CanonicalAddr,
}

pub const CONFIG: Item<Config> = Item::new("config");
```

---

## Contract: `terraswap_router` — contract.rs (key structures)

Key functions:
- `instantiate` — stores Config with 3 factory addresses (terraswap, loop, astroport)
- `execute` — dispatches Receive, ExecuteSwapOperations, ExecuteSwapOperation, AssertMinimumReceive
- `execute_swap_operations` — chains multiple swap operations via self-calling WasmMsg::Execute
- `simulate_swap_operations` — simulates multi-hop swaps including native swaps and tax deductions
- `reverse_simulate_swap_operations` — reverse multi-hop simulation
- `assert_operations` — validates operation chain has single output token

---

## Contract: `terraswap_router` — operations.rs

Key function:
- `execute_swap_operation` — executes a single swap (NativeSwap via TerraMsg, or TerraSwap/Loop/Astroport via pair contract)
- `asset_into_swap_msg` — converts Asset into proper CosmosMsg for swap (native with funds, CW20 via Send)

---

## Contract: `terraswap_router` — querier.rs

```rust
pub fn compute_tax(
    querier: &QuerierWrapper<TerraQuery>,
    amount: Uint128,
    denom: String,
) -> StdResult<Uint128> {
    // IBC tokens have zero tax
    // Uses TerraQuerier to get tax_rate and tax_cap
    // Returns min(amount - amount * DECIMAL_FRACTION / (DECIMAL_FRACTION * tax_rate + DECIMAL_FRACTION), tax_cap)
}

pub fn compute_reverse_tax(
    querier: &QuerierWrapper<TerraQuery>,
    amount: Uint128,
    denom: String,
) -> StdResult<Uint128> {
    // Reverse tax calculation
}
```

---

## Summary of Key Type Definitions

### Core Types (package level)

| Type | Fields |
|------|--------|
| `Asset` | `info: AssetInfo`, `amount: Uint128` |
| `AssetInfo` (enum) | `Token { contract_addr: String }`, `NativeToken { denom: String }` |
| `AssetRaw` | `info: AssetInfoRaw`, `amount: Uint128` |
| `AssetInfoRaw` (enum) | `Token { contract_addr: CanonicalAddr }`, `NativeToken { denom: String }` |
| `PairInfo` | `asset_infos: [AssetInfo; 2]`, `contract_addr: String`, `liquidity_token: String`, `asset_decimals: [u8; 2]` |
| `PairInfoRaw` | `asset_infos: [AssetInfoRaw; 2]`, `contract_addr: CanonicalAddr`, `liquidity_token: CanonicalAddr`, `asset_decimals: [u8; 2]` |

### Pair Messages

| Message | Variants/Fields |
|---------|----------------|
| `pair::InstantiateMsg` | `asset_infos: [AssetInfo; 2]`, `token_code_id: u64`, `asset_decimals: [u8; 2]` |
| `pair::ExecuteMsg` | `Receive(Cw20ReceiveMsg)`, `ProvideLiquidity { assets, receiver, deadline, slippage_tolerance }`, `Swap { offer_asset, belief_price, max_spread, to, deadline }` |
| `pair::Cw20HookMsg` | `Swap { belief_price, max_spread, to, deadline }`, `WithdrawLiquidity { min_assets, deadline }` |
| `pair::QueryMsg` | `Pair {}`, `Pool {}`, `Simulation { offer_asset }`, `ReverseSimulation { ask_asset }` |
| `pair::PoolResponse` | `assets: [Asset; 2]`, `total_share: Uint128` |
| `pair::SimulationResponse` | `return_amount`, `spread_amount`, `commission_amount` (all Uint128) |
| `pair::ReverseSimulationResponse` | `offer_amount`, `spread_amount`, `commission_amount` (all Uint128) |

### Factory Messages

| Message | Variants/Fields |
|---------|----------------|
| `factory::InstantiateMsg` | `pair_code_id: u64`, `token_code_id: u64` |
| `factory::ExecuteMsg` | `UpdateConfig { owner, token_code_id, pair_code_id }`, `CreatePair { assets }`, `AddNativeTokenDecimals { denom, decimals }`, `MigratePair { contract, code_id }` |
| `factory::QueryMsg` | `Config {}`, `Pair { asset_infos }`, `Pairs { start_after, limit }`, `NativeTokenDecimals { denom }` |
| `factory::ConfigResponse` | `owner: String`, `pair_code_id: u64`, `token_code_id: u64` |
| `factory::PairsResponse` | `pairs: Vec<PairInfo>` |

### Router Messages

| Message | Variants/Fields |
|---------|----------------|
| `router::InstantiateMsg` | `terraswap_factory: String`, `loop_factory: String`, `astroport_factory: String` |
| `router::SwapOperation` (enum) | `NativeSwap { offer_denom, ask_denom }`, `TerraSwap { offer_asset_info, ask_asset_info }`, `Loop { offer_asset_info, ask_asset_info }`, `Astroport { offer_asset_info, ask_asset_info }` |
| `router::ExecuteMsg` | `Receive`, `ExecuteSwapOperations { operations, minimum_receive, to, deadline }`, `ExecuteSwapOperation { operation, to, deadline }`, `AssertMinimumReceive { asset_info, prev_balance, minimum_receive, receiver }` |
| `router::QueryMsg` | `Config {}`, `SimulateSwapOperations { offer_amount, operations }`, `ReverseSimulateSwapOperations { ask_amount, operations }` |
| `router::ConfigResponse` | `terraswap_factory`, `loop_factory`, `astroport_factory` (all String) |

### State (contract level)

| Contract | Storage Key | Type |
|----------|------------|------|
| Factory | `"config"` | `Config { owner: CanonicalAddr, pair_code_id: u64, token_code_id: u64 }` |
| Factory | `"tmp_pair_info"` | `TmpPairInfo { pair_key: Vec<u8>, assets: [AssetRaw; 2], asset_decimals: [u8; 2], sender: Addr }` |
| Factory | `"pair_info"` (Map) | `PairInfoRaw` keyed by sorted asset info bytes |
| Factory | `"allow_native_token"` (Map) | `u8` (decimals) keyed by denom bytes |
| Pair | `"pair_info"` | `PairInfoRaw` |
| Router | `"config"` | `Config { terraswap_factory: CanonicalAddr, loop_factory: CanonicalAddr, astroport_factory: CanonicalAddr }` |

### Constants

| Constant | Value | Location |
|----------|-------|----------|
| `COMMISSION_RATE` | `3` (used as permille = 0.3%) | Pair contract |
| `MINIMUM_LIQUIDITY_AMOUNT` | `1_000` | Pair contract |
| `MAX_LIMIT` (pagination) | `30` | Factory state |
| `DEFAULT_LIMIT` (pagination) | `10` | Factory state |
