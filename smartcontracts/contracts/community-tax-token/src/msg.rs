use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Decimal, Uint128};
use cw20::{Cw20Coin, Cw20ReceiveMsg, Expiration};
use cw20_base::msg::InstantiateMarketingInfo;

/// Exact UST1 invoice (6 dp) for one SKU unlock **or** one settings batch.
pub const INVOICE_UST1: u128 = 50_000_000;
/// Recommended / enforced per-leg and combined instantiate cap (25%).
pub const MAX_TAX_BPS: u16 = 2_500;
pub const BPS_DENOM: u16 = 10_000;
pub const MAX_SINKS: usize = 4;
pub const MAX_DECIMALS: u8 = 18;
/// Retail + on-chain floor so human-scale Swap/Trade/tape math stays sane (#604).
pub const MIN_DECIMALS: u8 = 6;
pub const MIN_NAME_LEN: usize = 3;
pub const MAX_NAME_LEN: usize = 50;
pub const MIN_SYMBOL_LEN: usize = 3;
pub const MAX_SYMBOL_LEN: usize = 12;
/// Manager exemptions written at instantiate when `ExemptionDirectory` is purchased (#605).
pub const MAX_INITIAL_EXEMPT: usize = 20;

/// Paid feature SKUs. MintControl is instantiate-only.
#[cw_serde]
pub enum Sku {
    MintControl,
    TransferTax,
    SplitRouter,
    AutoV2Lp,
    ExemptionDirectory,
    VariableRates,
    LaunchGuards,
}

impl Sku {
    pub fn as_str(&self) -> &'static str {
        match self {
            Sku::MintControl => "mint_control",
            Sku::TransferTax => "transfer_tax",
            Sku::SplitRouter => "split_router",
            Sku::AutoV2Lp => "auto_v2_lp",
            Sku::ExemptionDirectory => "exemption_directory",
            Sku::VariableRates => "variable_rates",
            Sku::LaunchGuards => "launch_guards",
        }
    }
}

#[cw_serde]
pub enum SinkKind {
    Treasury,
    Burn,
    AutoLp,
    Wallet,
}

#[cw_serde]
pub struct Sink {
    pub kind: SinkKind,
    /// Required when `kind == Wallet`.
    pub addr: Option<String>,
    pub bps: u16,
}

#[cw_serde]
pub struct LaunchGuardsConfig {
    pub max_wallet: Option<Uint128>,
    pub cooldown_blocks: u64,
    pub trading_enabled: bool,
}

#[cw_serde]
pub struct AutoLpConfig {
    pub pair: Option<String>,
    pub threshold: Uint128,
    pub lp_recipient: String,
    /// Optional skim floor forwarded to the sister (`UpdateConfig`). Omit → merge.
    #[serde(default)]
    pub skim_max_spread: Option<Decimal>,
    #[serde(default)]
    pub skim_min_return: Option<Uint128>,
}

/// UST1 `Send` hook on this token (or forwarded by the launcher).
#[cw_serde]
#[allow(clippy::large_enum_variant)]
pub enum InvoiceHookMsg {
    EnableFeature { sku: Sku },
    UpdateSettings { settings: SettingsBatch },
}

/// One atomic manager save. Invoice is always [`INVOICE_UST1`] for the whole batch.
/// Keys whose SKU is not unlocked fail the entire batch.
#[cw_serde]
#[derive(Default)]
pub struct SettingsBatch {
    pub buy_bps: Option<u16>,
    pub sell_bps: Option<u16>,
    pub treasury: Option<String>,
    pub transfer_bps: Option<u16>,
    pub sinks: Option<Vec<Sink>>,
    pub add_exempt: Option<Vec<String>>,
    pub remove_exempt: Option<Vec<String>>,
    pub autolp: Option<AutoLpConfig>,
    pub launch_guards: Option<LaunchGuardsConfig>,
    pub minter: Option<String>,
    pub revoke_mint: Option<bool>,
}

impl SettingsBatch {
    pub fn is_empty(&self) -> bool {
        self.buy_bps.is_none()
            && self.sell_bps.is_none()
            && self.treasury.is_none()
            && self.transfer_bps.is_none()
            && self.sinks.is_none()
            && self.add_exempt.is_none()
            && self.remove_exempt.is_none()
            && self.autolp.is_none()
            && self.launch_guards.is_none()
            && self.minter.is_none()
            && self.revoke_mint.is_none()
    }
}

#[cw_serde]
pub struct InstantiateMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Vec<Cw20Coin>,
    pub marketing: Option<InstantiateMarketingInfo>,
    pub manager: String,
    pub treasury: String,
    pub buy_bps: u16,
    pub sell_bps: u16,
    pub max_buy_bps: u16,
    pub max_sell_bps: u16,
    pub max_transfer_bps: u16,
    /// Factory used to verify `RegisterListedPair`.
    pub factory: String,
    pub router: Option<String>,
    pub ust1: String,
    pub cmm_treasury: String,
    pub features: Vec<Sku>,
    pub mint: Option<MintInit>,
    pub transfer_bps: Option<u16>,
    pub sinks: Option<Vec<Sink>>,
    pub autolp: Option<String>,
    pub launcher: Option<String>,
    pub launch_guards: Option<LaunchGuardsConfig>,
    /// Written to `MANAGER_EXEMPT` when `ExemptionDirectory` is in `features` (#605).
    #[serde(default)]
    pub initial_exempt: Option<Vec<String>>,
}

#[cw_serde]
pub struct MintInit {
    pub minter: String,
    pub cap: Option<Uint128>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Transfer {
        recipient: String,
        amount: Uint128,
    },
    Burn {
        amount: Uint128,
    },
    Send {
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    IncreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    DecreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    TransferFrom {
        owner: String,
        recipient: String,
        amount: Uint128,
    },
    SendFrom {
        owner: String,
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    BurnFrom {
        owner: String,
        amount: Uint128,
    },
    /// Issuing supply. Requires MintControl SKU and a live minter. **Not** a settings invoice.
    Mint {
        recipient: String,
        amount: Uint128,
    },
    /// UST1 invoice: [`InvoiceHookMsg`].
    Receive(Cw20ReceiveMsg),
    /// Permissionless. Factory-listed pair that holds this token as an asset.
    RegisterListedPair {
        pair: String,
    },
    /// Launcher-only. Bind the AutoLP sister after it is instantiated in the create/enable reply (#605).
    BindAutolp {
        autolp: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cw20::BalanceResponse)]
    Balance { address: String },
    #[returns(cw20::TokenInfoResponse)]
    TokenInfo {},
    #[returns(Option<cw20::MinterResponse>)]
    Minter {},
    #[returns(cw20::AllowanceResponse)]
    Allowance { owner: String, spender: String },
    #[returns(cw20::AllAllowancesResponse)]
    AllAllowances {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(cw20::AllSpenderAllowancesResponse)]
    AllSpenderAllowances {
        spender: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(cw20::AllAccountsResponse)]
    AllAccounts {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(cw20::MarketingInfoResponse)]
    MarketingInfo {},
    #[returns(cw20::DownloadLogoResponse)]
    DownloadLogo {},
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(FeaturesResponse)]
    GetFeatures {},
    #[returns(ExemptionsResponse)]
    GetExemptions {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(IsExemptResponse)]
    IsProtocolExempt { address: String },
    /// dApp max-spend. Optional `send_msg` classifies a CW20 `Send` hook (sell = pair `Swap`).
    #[returns(TaxPreviewResponse)]
    TaxPreview {
        from: String,
        to: String,
        amount: Uint128,
        send_msg: Option<Binary>,
    },
    #[returns(LauncherOriginResponse)]
    GetLauncherOrigin {},
    /// Present after a successful foreign adopt (#626). Launcher-created tokens return empty.
    #[returns(MigrateOriginResponse)]
    GetMigrateOrigin {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub manager: Addr,
    pub treasury: Addr,
    pub buy_bps: u16,
    pub sell_bps: u16,
    pub transfer_bps: u16,
    pub max_buy_bps: u16,
    pub max_sell_bps: u16,
    pub max_transfer_bps: u16,
    pub factory: Addr,
    pub router: Option<Addr>,
    pub ust1: Addr,
    pub cmm_treasury: Addr,
    pub autolp: Option<Addr>,
    pub sinks: Vec<SinkView>,
    pub launch_guards: Option<LaunchGuardsView>,
    pub mint_revoked: bool,
}

#[cw_serde]
pub struct SinkView {
    pub kind: SinkKind,
    pub addr: Option<Addr>,
    pub bps: u16,
}

#[cw_serde]
pub struct LaunchGuardsView {
    pub max_wallet: Option<Uint128>,
    pub cooldown_blocks: u64,
    pub trading_enabled: bool,
}

#[cw_serde]
pub struct FeaturesResponse {
    pub mint_control: bool,
    pub transfer_tax: bool,
    pub split_router: bool,
    pub auto_v2_lp: bool,
    pub exemption_directory: bool,
    pub variable_rates: bool,
    pub launch_guards: bool,
}

#[cw_serde]
pub struct ExemptionsResponse {
    pub protocol: Vec<Addr>,
    pub manager: Vec<Addr>,
}

#[cw_serde]
pub struct IsExemptResponse {
    pub address: Addr,
    pub protocol: bool,
    pub manager: bool,
}

#[cw_serde]
pub enum TaxKind {
    Honest,
    Buy,
    Sell,
    Transfer,
}

#[cw_serde]
pub struct TaxPreviewResponse {
    pub kind: TaxKind,
    pub declared: Uint128,
    /// Amount debited from `from` (pair-direct sell = declared + tax; router sell = declared).
    pub debit: Uint128,
    /// Amount credited to `to` (buy/transfer = declared − tax).
    pub credit: Uint128,
    pub tax: Uint128,
    /// Authenticated hop trader when `from` is the official router (**T592-13** / #607).
    #[serde(default)]
    pub hop_trader: Option<Addr>,
    /// Extra-debit taken from [`Self::hop_trader`] on a router sell (pair still credited `declared`).
    #[serde(default)]
    pub hop_trader_debit: Option<Uint128>,
}

#[cw_serde]
pub struct LauncherOriginResponse {
    pub launcher: Option<Addr>,
}

#[cw_serde]
pub struct MigrateOriginResponse {
    pub source_cw2: Option<String>,
    pub source_version: Option<String>,
    pub source_code_id: Option<u64>,
    pub migrated_at_height: Option<u64>,
}

/// Same-crate bump is `{}` / `{ "adopt": null }`. Foreign adopt sets [`AdoptMigrateMsg`].
#[cw_serde]
pub struct MigrateMsg {
    #[serde(default)]
    pub adopt: Option<AdoptMigrateMsg>,
}

/// Free-profile adopt onto this wasm. No paid SKUs. Source admin becomes `manager`.
#[cw_serde]
pub struct AdoptMigrateMsg {
    pub manager: String,
    pub treasury: String,
    pub factory: String,
    pub router: Option<String>,
    pub ust1: String,
    pub cmm_treasury: String,
    /// Official launcher — written to `CONFIG.launcher` so `GetLauncherOrigin` matches catalog env.
    pub official_launcher: String,
    pub buy_bps: u16,
    pub sell_bps: u16,
    pub transfer_bps: Option<u16>,
    pub max_buy_bps: u16,
    pub max_sell_bps: u16,
    pub max_transfer_bps: u16,
    /// Informational LCD source id (cw2 allowlist is authoritative).
    #[serde(default)]
    pub source_code_id: Option<u64>,
}
