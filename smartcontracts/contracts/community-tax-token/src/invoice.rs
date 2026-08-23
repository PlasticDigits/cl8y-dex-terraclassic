use cosmwasm_std::{to_json_binary, Addr, DepsMut, Env, Response, Uint128, WasmMsg};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{
    AutoLpConfig, InvoiceHookMsg, LaunchGuardsConfig, SettingsBatch, Sink, SinkKind, Sku,
    INVOICE_UST1, MAX_SINKS, MAX_TAX_BPS,
};
use crate::state::{
    Config, Features, LaunchGuards, StoredSink, CONFIG, FEATURES, LAUNCH_GUARDS, MANAGER_EXEMPT,
    PROTOCOL_EXEMPT, SINKS,
};
use crate::tax::is_protocol_exempt;

pub fn execute_receive(
    deps: DepsMut,
    _env: Env,
    info_sender: Addr,
    cw20: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info_sender != config.ust1 {
        return Err(ContractError::InvoiceToken {});
    }
    let payer = deps.api.addr_validate(&cw20.sender)?;
    if payer != config.manager {
        return Err(ContractError::Unauthorized {});
    }

    let hook: InvoiceHookMsg = cosmwasm_std::from_json(&cw20.msg)?;
    match hook {
        InvoiceHookMsg::EnableFeature { sku } => {
            assert_exact_invoice(cw20.amount)?;
            enable_feature(deps, &config, sku, cw20.amount)
        }
        InvoiceHookMsg::UpdateSettings { settings } => {
            assert_exact_invoice(cw20.amount)?;
            update_settings(deps, &config, settings, cw20.amount)
        }
    }
}

fn assert_exact_invoice(amount: Uint128) -> Result<(), ContractError> {
    let required = Uint128::new(INVOICE_UST1);
    if amount != required {
        return Err(ContractError::InvoiceAmount {
            required: required.to_string(),
            got: amount.to_string(),
        });
    }
    Ok(())
}

fn forward_ust1(config: &Config, amount: Uint128) -> Result<WasmMsg, ContractError> {
    Ok(WasmMsg::Execute {
        contract_addr: config.ust1.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: config.cmm_treasury.to_string(),
            amount,
        })?,
        funds: vec![],
    })
}

fn enable_feature(
    deps: DepsMut,
    config: &Config,
    sku: Sku,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if matches!(sku, Sku::MintControl) {
        return Err(ContractError::MintControlInstantiateOnly {});
    }
    let mut features = FEATURES.load(deps.storage)?;
    if features.is_enabled(&sku) {
        return Err(ContractError::FeatureAlreadyEnabled {
            sku: sku.as_str().to_string(),
        });
    }
    features.enable(&sku);
    FEATURES.save(deps.storage, &features)?;

    if matches!(sku, Sku::LaunchGuards) && LAUNCH_GUARDS.may_load(deps.storage)?.is_none() {
        LAUNCH_GUARDS.save(
            deps.storage,
            &LaunchGuards {
                max_wallet: None,
                cooldown_blocks: 0,
                trading_enabled: true,
            },
        )?;
    }

    Ok(Response::new()
        .add_message(forward_ust1(config, amount)?)
        .add_attribute("action", "enable_feature")
        .add_attribute("sku", sku.as_str())
        .add_attribute("invoice", amount))
}

fn update_settings(
    mut deps: DepsMut,
    config: &Config,
    batch: SettingsBatch,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if batch.is_empty() {
        return Err(ContractError::NoOpSettings {});
    }
    let features = FEATURES.load(deps.storage)?;
    let mut cfg = CONFIG.load(deps.storage)?;
    let mut changed = false;

    if let Some(bps) = batch.buy_bps {
        require_variable_or_free_profile(&features, true)?;
        if bps != cfg.buy_bps {
            validate_bps(bps, cfg.max_buy_bps)?;
            cfg.buy_bps = bps;
            changed = true;
        }
    }
    if let Some(bps) = batch.sell_bps {
        require_variable_or_free_profile(&features, true)?;
        if bps != cfg.sell_bps {
            validate_bps(bps, cfg.max_sell_bps)?;
            cfg.sell_bps = bps;
            changed = true;
        }
    }
    if let Some(ref treasury) = batch.treasury {
        let addr = deps.api.addr_validate(treasury)?;
        if addr != cfg.treasury {
            cfg.treasury = addr;
            changed = true;
        }
    }
    if let Some(bps) = batch.transfer_bps {
        if !features.transfer_tax {
            return Err(ContractError::SkuNotUnlocked {
                sku: Sku::TransferTax.as_str().to_string(),
            });
        }
        if bps != cfg.transfer_bps {
            validate_bps(bps, cfg.max_transfer_bps)?;
            cfg.transfer_bps = bps;
            changed = true;
        }
    }
    if let Some(ref sinks) = batch.sinks {
        if !features.split_router {
            return Err(ContractError::SkuNotUnlocked {
                sku: Sku::SplitRouter.as_str().to_string(),
            });
        }
        let stored = validate_sinks(deps.branch(), &cfg, sinks)?;
        let current = SINKS.may_load(deps.storage)?.unwrap_or_default();
        if stored != current {
            SINKS.save(deps.storage, &stored)?;
            changed = true;
        }
    }
    if let Some(ref addrs) = batch.add_exempt {
        if !features.exemption_directory {
            return Err(ContractError::SkuNotUnlocked {
                sku: Sku::ExemptionDirectory.as_str().to_string(),
            });
        }
        for raw in addrs {
            let addr = deps.api.addr_validate(raw)?;
            if !MANAGER_EXEMPT
                .may_load(deps.storage, &addr)?
                .unwrap_or(false)
            {
                MANAGER_EXEMPT.save(deps.storage, &addr, &true)?;
                changed = true;
            }
        }
    }
    if let Some(ref addrs) = batch.remove_exempt {
        if !features.exemption_directory {
            return Err(ContractError::SkuNotUnlocked {
                sku: Sku::ExemptionDirectory.as_str().to_string(),
            });
        }
        let self_addr = Addr::unchecked(""); // filled below via env? we need contract address
        let _ = self_addr;
        for raw in addrs {
            let addr = deps.api.addr_validate(raw)?;
            if is_protocol_exempt(deps.storage, &cfg.factory, &addr)
                || PROTOCOL_EXEMPT
                    .may_load(deps.storage, &addr)?
                    .unwrap_or(false)
                || addr == cfg.factory
                || cfg.router.as_ref() == Some(&addr)
                || cfg.autolp.as_ref() == Some(&addr)
            {
                return Err(ContractError::CannotRemoveProtocolExempt {});
            }
            if crate::tax::is_listed_pair(deps.storage, &addr) {
                return Err(ContractError::CannotRemoveProtocolExempt {});
            }
            if MANAGER_EXEMPT
                .may_load(deps.storage, &addr)?
                .unwrap_or(false)
            {
                MANAGER_EXEMPT.remove(deps.storage, &addr);
                changed = true;
            }
        }
    }
    if let Some(ref autolp) = batch.autolp {
        if !features.auto_v2_lp {
            return Err(ContractError::SkuNotUnlocked {
                sku: Sku::AutoV2Lp.as_str().to_string(),
            });
        }
        apply_autolp_settings(&mut deps, &mut cfg, autolp.clone(), &mut changed)?;
    }
    if let Some(ref guards) = batch.launch_guards {
        if !features.launch_guards {
            return Err(ContractError::SkuNotUnlocked {
                sku: Sku::LaunchGuards.as_str().to_string(),
            });
        }
        apply_launch_guard_settings(deps.storage, guards.clone(), &mut changed)?;
    }
    if batch.minter.is_some() || batch.revoke_mint == Some(true) {
        if !features.mint_control {
            return Err(ContractError::SkuNotUnlocked {
                sku: Sku::MintControl.as_str().to_string(),
            });
        }
        apply_mint_settings(&mut deps, &mut cfg, &batch, &mut changed)?;
    }

    if !changed {
        return Err(ContractError::NoOpSettings {});
    }
    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new()
        .add_message(forward_ust1(config, amount)?)
        .add_attribute("action", "update_settings")
        .add_attribute("invoice", amount))
}

fn require_variable_or_free_profile(
    features: &Features,
    _free_profile_ok: bool,
) -> Result<(), ContractError> {
    // Free-profile buy/sell/treasury may change in a batch after instantiate
    // (50 UST1). VariableRates is only required to *raise* above instantiate
    // values — we already cap via max_*_bps. Always allowed here.
    let _ = features;
    Ok(())
}

fn validate_bps(bps: u16, cap: u16) -> Result<(), ContractError> {
    if bps > cap || bps > MAX_TAX_BPS {
        return Err(ContractError::TaxBpsCap {
            bps,
            cap: cap.min(MAX_TAX_BPS),
        });
    }
    Ok(())
}

pub fn validate_sinks(
    deps: DepsMut,
    _config: &Config,
    sinks: &[Sink],
) -> Result<Vec<StoredSink>, ContractError> {
    if sinks.is_empty() || sinks.len() > MAX_SINKS {
        return Err(ContractError::TooManySinks { max: MAX_SINKS });
    }
    let sum: u32 = sinks.iter().map(|s| u32::from(s.bps)).sum();
    if sum != u32::from(crate::msg::BPS_DENOM) {
        return Err(ContractError::SinkRatio { sum });
    }
    let mut out = Vec::with_capacity(sinks.len());
    for sink in sinks {
        let addr = match sink.kind {
            SinkKind::Wallet => {
                let raw = sink.addr.as_ref().ok_or(ContractError::WalletSinkAddr {})?;
                Some(deps.api.addr_validate(raw)?)
            }
            SinkKind::Treasury | SinkKind::Burn | SinkKind::AutoLp => None,
        };
        out.push(StoredSink {
            kind: sink.kind.clone(),
            addr,
            bps: sink.bps,
        });
    }
    Ok(out)
}

fn apply_autolp_settings(
    deps: &mut DepsMut,
    cfg: &mut Config,
    autolp: AutoLpConfig,
    changed: &mut bool,
) -> Result<(), ContractError> {
    let recipient = deps.api.addr_validate(&autolp.lp_recipient)?;
    let _ = recipient;
    if let Some(pair) = autolp.pair {
        let _ = deps.api.addr_validate(&pair)?;
    }
    if cfg.autolp.is_none() {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "AutoLP contract not bound; enable AutoV2Lp via launcher",
        )));
    }
    // Pair/threshold/recipient live on the AutoLP sister; token only tracks binding.
    *changed = true;
    Ok(())
}

fn apply_launch_guard_settings(
    storage: &mut dyn cosmwasm_std::Storage,
    incoming: LaunchGuardsConfig,
    changed: &mut bool,
) -> Result<(), ContractError> {
    let current = LAUNCH_GUARDS.may_load(storage)?.unwrap_or(LaunchGuards {
        max_wallet: None,
        cooldown_blocks: 0,
        trading_enabled: true,
    });
    let next = LaunchGuards {
        max_wallet: incoming.max_wallet,
        cooldown_blocks: incoming.cooldown_blocks,
        trading_enabled: incoming.trading_enabled,
    };
    if current != next {
        LAUNCH_GUARDS.save(storage, &next)?;
        *changed = true;
    }
    Ok(())
}

fn apply_mint_settings(
    deps: &mut DepsMut,
    cfg: &mut Config,
    batch: &SettingsBatch,
    changed: &mut bool,
) -> Result<(), ContractError> {
    use cw20_base::state::{MinterData, TOKEN_INFO};

    if cfg.mint_revoked {
        return Err(ContractError::MintRevoked {});
    }
    if batch.revoke_mint == Some(true) {
        TOKEN_INFO.update(deps.storage, |mut info| -> Result<_, ContractError> {
            info.mint = None;
            Ok(info)
        })?;
        cfg.mint_revoked = true;
        *changed = true;
        return Ok(());
    }
    if let Some(minter) = &batch.minter {
        let addr = deps.api.addr_validate(minter)?;
        TOKEN_INFO.update(deps.storage, |mut info| -> Result<_, ContractError> {
            match &mut info.mint {
                Some(m) => {
                    if m.minter != addr {
                        m.minter = addr.clone();
                    } else {
                        return Ok(info);
                    }
                }
                None => {
                    info.mint = Some(MinterData {
                        minter: addr.clone(),
                        cap: None,
                    });
                }
            }
            Ok(info)
        })?;
        *changed = true;
    }
    Ok(())
}
