use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

use crate::msg::{SinkKind, Sku};

#[cw_serde]
pub struct Config {
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
    pub launcher: Option<Addr>,
    pub mint_revoked: bool,
}

#[cw_serde]
pub struct Features {
    pub mint_control: bool,
    pub transfer_tax: bool,
    pub split_router: bool,
    pub auto_v2_lp: bool,
    pub exemption_directory: bool,
    pub variable_rates: bool,
    pub launch_guards: bool,
}

impl Features {
    pub fn from_skus(skus: &[Sku]) -> Self {
        let mut f = Features {
            mint_control: false,
            transfer_tax: false,
            split_router: false,
            auto_v2_lp: false,
            exemption_directory: false,
            variable_rates: false,
            launch_guards: false,
        };
        for sku in skus {
            match sku {
                Sku::MintControl => f.mint_control = true,
                Sku::TransferTax => f.transfer_tax = true,
                Sku::SplitRouter => f.split_router = true,
                Sku::AutoV2Lp => f.auto_v2_lp = true,
                Sku::ExemptionDirectory => f.exemption_directory = true,
                Sku::VariableRates => f.variable_rates = true,
                Sku::LaunchGuards => f.launch_guards = true,
            }
        }
        f
    }

    pub fn is_enabled(&self, sku: &Sku) -> bool {
        match sku {
            Sku::MintControl => self.mint_control,
            Sku::TransferTax => self.transfer_tax,
            Sku::SplitRouter => self.split_router,
            Sku::AutoV2Lp => self.auto_v2_lp,
            Sku::ExemptionDirectory => self.exemption_directory,
            Sku::VariableRates => self.variable_rates,
            Sku::LaunchGuards => self.launch_guards,
        }
    }

    pub fn enable(&mut self, sku: &Sku) {
        match sku {
            Sku::MintControl => self.mint_control = true,
            Sku::TransferTax => self.transfer_tax = true,
            Sku::SplitRouter => self.split_router = true,
            Sku::AutoV2Lp => self.auto_v2_lp = true,
            Sku::ExemptionDirectory => self.exemption_directory = true,
            Sku::VariableRates => self.variable_rates = true,
            Sku::LaunchGuards => self.launch_guards = true,
        }
    }
}

#[cw_serde]
pub struct StoredSink {
    pub kind: SinkKind,
    pub addr: Option<Addr>,
    pub bps: u16,
}

#[cw_serde]
pub struct LaunchGuards {
    pub max_wallet: Option<Uint128>,
    pub cooldown_blocks: u64,
    pub trading_enabled: bool,
}

pub const CONFIG: Item<Config> = Item::new("cfg");
pub const FEATURES: Item<Features> = Item::new("feat");
pub const SINKS: Item<Vec<StoredSink>> = Item::new("sinks");
pub const LAUNCH_GUARDS: Item<LaunchGuards> = Item::new("lg");

/// Factory-verified listed pairs (protocol-exempt; cannot be removed).
pub const LISTED_PAIRS: Map<&Addr, bool> = Map::new("pairs");
/// Extra protocol addresses (router, autolp, factory) — cannot be removed.
pub const PROTOCOL_EXEMPT: Map<&Addr, bool> = Map::new("pex");
/// Manager directory (ExemptionDirectory SKU).
pub const MANAGER_EXEMPT: Map<&Addr, bool> = Map::new("mex");
/// Last taxed swap block per **user** wallet (LaunchGuards cooldown, H608-1 / #608).
/// Listed pairs / protocol-exempt addresses are never written or checked.
pub const LAST_TRADE_BLOCK: Map<&Addr, u64> = Map::new("ltb");

/// Written only by the foreign adopt importer (#626). Key does not collide with cw20-base.
#[cw_serde]
pub struct MigrateOrigin {
    pub source_cw2: String,
    pub source_version: String,
    pub source_code_id: Option<u64>,
    pub migrated_at_height: u64,
}

pub const MIGRATE_ORIGIN: Item<MigrateOrigin> = Item::new("mig_origin");
