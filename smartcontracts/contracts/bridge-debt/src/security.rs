//! Market trust and issuance policy. Defaults only affect future registrations.
use super::*;
use crate::msg::MigrateMsg;

pub(super) fn contract_code(deps: Deps, address: &Addr) -> Result<u64, ContractError> {
    Ok(deps.querier.query_wasm_contract_info(address)?.code_id)
}

pub(super) fn inspect_underlying(
    deps: Deps,
    env: &Env,
    underlying: &Addr,
    debt_code_id: u64,
    underlying_code_id: u64,
    multi_minter: bool,
) -> Result<MarketPins, ContractError> {
    if contract_code(deps, underlying)? != underlying_code_id {
        return Err(ContractError::UnapprovedImplementation {});
    }
    underlying_decimals(deps, underlying)?;
    let minter: MinterResponse = deps
        .querier
        .query_wasm_smart(underlying, &Cw20QueryMsg::Minter {})
        .map_err(|_| ContractError::UnderlyingMinterMismatch {})?;
    let primary = deps.api.addr_validate(&minter.minter)?;
    let mut additional = if multi_minter {
        // Approved multi-minter implementations must support a complete bounded inventory.
        // Implementations may clamp page sizes to 30; probe the next page explicitly.
        let listed: MintersProbe = deps
            .querier
            .query_wasm_smart(
                underlying,
                &UnderlyingProbe::Minters {
                    start_after: None,
                    limit: Some(30),
                },
            )
            .map_err(|_| ContractError::UnderlyingMinterMismatch {})?;
        if listed.minters.len() > 30 {
            return Err(ContractError::UnderlyingMinterMismatch {});
        }
        if listed.minters.len() == 30 {
            let next: MintersProbe = deps
                .querier
                .query_wasm_smart(
                    underlying,
                    &UnderlyingProbe::Minters {
                        start_after: listed.minters.last().cloned(),
                        limit: Some(1),
                    },
                )
                .map_err(|_| ContractError::UnderlyingMinterMismatch {})?;
            if !next.minters.is_empty() {
                return Err(ContractError::UnderlyingMinterMismatch {});
            }
        }
        listed
            .minters
            .iter()
            .map(|a| deps.api.addr_validate(a))
            .collect::<StdResult<Vec<_>>>()?
    } else {
        vec![]
    };
    additional.sort();
    let before = additional.len();
    additional.dedup();
    if additional.len() != before
        || (primary != env.contract.address && !additional.contains(&env.contract.address))
    {
        return Err(ContractError::UnderlyingMinterMismatch {});
    }
    Ok(MarketPins {
        debt_code_id,
        underlying_code_id,
        multi_minter,
        minter: primary,
        minter_cap: minter.cap,
        additional_minters: additional,
    })
}

#[cosmwasm_schema::cw_serde]
enum UnderlyingProbe {
    Minters {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

pub(super) fn approve_underlying(
    deps: DepsMut,
    info: MessageInfo,
    code_id: u64,
    multi_minter: bool,
) -> Result<Response, ContractError> {
    must_owner(&CONFIG.load(deps.storage)?, &info)?;
    if code_id == 0 {
        return Err(ContractError::UnapprovedImplementation {});
    }
    UNDERLYING_CODES.save(deps.storage, code_id, &UnderlyingPolicy { multi_minter })?;
    Ok(Response::new()
        .add_attribute("action", "approve_underlying_code")
        .add_attribute("code_id", code_id.to_string()))
}

pub(super) fn configure_market(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    underlying: String,
    limits: IssuanceLimits,
    paused: bool,
) -> Result<Response, ContractError> {
    must_owner(&CONFIG.load(deps.storage)?, &info)?;
    let underlying = deps.api.addr_validate(&underlying)?;
    let mut market = load_market(deps.as_ref(), &underlying)?;
    if limits.epoch_seconds == 0
        || limits.max_per_issue.is_zero()
        || limits.max_outstanding.is_zero()
        || limits.max_per_epoch.is_zero()
        || market
            .limits
            .as_ref()
            .is_some_and(|old| old.epoch_seconds != limits.epoch_seconds)
    {
        return Err(ContractError::InvalidLimits {});
    }
    // Reconfiguration never refunds consumed quota; epochs have immutable duration.
    if market.limits.is_none() {
        market.epoch = env.block.time.seconds() / limits.epoch_seconds;
    }
    market.limits = Some(limits);
    market.paused = paused;
    MARKETS.save(deps.storage, &underlying, &market)?;
    Ok(Response::new().add_attribute("action", "configure_market"))
}

pub(super) fn consume_limit(
    market: &mut Market,
    env: &Env,
    amount: Uint128,
) -> Result<(), ContractError> {
    let limits = market
        .limits
        .as_ref()
        .ok_or(ContractError::IssuanceDisabled {})?;
    if market.paused || market.pending_upgrade.is_some() {
        return Err(ContractError::IssuanceDisabled {});
    }
    let epoch = env.block.time.seconds() / limits.epoch_seconds;
    if epoch < market.epoch {
        return Err(ContractError::InvalidLimits {});
    }
    if epoch > market.epoch {
        market.epoch = epoch;
        market.epoch_issued = Uint128::zero();
    }
    let issued = market
        .epoch_issued
        .checked_add(amount)
        .map_err(|_| ContractError::Overflow {})?;
    let outstanding = market
        .outstanding
        .checked_add(amount)
        .map_err(|_| ContractError::Overflow {})?;
    if amount > limits.max_per_issue
        || issued > limits.max_per_epoch
        || outstanding > limits.max_outstanding
    {
        return Err(ContractError::IssuanceLimit {});
    }
    market.epoch_issued = issued;
    Ok(())
}

pub(super) fn propose_upgrade(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    underlying: String,
    debt_code_id: u64,
    underlying_code_id: u64,
) -> Result<Response, ContractError> {
    must_owner(&CONFIG.load(deps.storage)?, &info)?;
    let underlying = deps.api.addr_validate(&underlying)?;
    let mut market = load_market(deps.as_ref(), &underlying)?;
    assert_market_consistent(deps.as_ref(), &env, &market)?;
    if debt_code_id == 0 || !UNDERLYING_CODES.has(deps.storage, underlying_code_id) {
        return Err(ContractError::UnapprovedImplementation {});
    }
    let token: TokenInfoResponse = deps
        .querier
        .query_wasm_smart(&underlying, &Cw20QueryMsg::TokenInfo {})?;
    market.pending_upgrade = Some(UpgradePins {
        debt_code_id,
        underlying_code_id,
        underlying_supply: token.total_supply,
    });
    MARKETS.save(deps.storage, &underlying, &market)?;
    Ok(Response::new().add_attribute("action", "propose_market_upgrade"))
}

pub(super) fn accept_upgrade(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    underlying: String,
) -> Result<Response, ContractError> {
    must_owner(&CONFIG.load(deps.storage)?, &info)?;
    let underlying = deps.api.addr_validate(&underlying)?;
    let mut market = load_market(deps.as_ref(), &underlying)?;
    let pending = market
        .pending_upgrade
        .clone()
        .ok_or(ContractError::NoPendingUpgrade {})?;
    let mut expected = market
        .pins
        .clone()
        .ok_or(ContractError::UnapprovedImplementation {})?;
    // A code-only coordinated upgrade cannot change debtor, minter set/cap, units or accounting.
    expected.debt_code_id = pending.debt_code_id;
    expected.underlying_code_id = pending.underlying_code_id;
    let policy = UNDERLYING_CODES.load(deps.storage, pending.underlying_code_id)?;
    if policy.multi_minter != expected.multi_minter {
        return Err(ContractError::UnderlyingMinterMismatch {});
    }
    let token: TokenInfoResponse = deps
        .querier
        .query_wasm_smart(&underlying, &Cw20QueryMsg::TokenInfo {})?;
    if token.total_supply != pending.underlying_supply {
        return Err(ContractError::DebtSupplyDiverged {});
    }
    market.pins = Some(expected);
    assert_market_consistent(deps.as_ref(), &env, &market)?;
    market.pending_upgrade = None;
    MARKETS.save(deps.storage, &underlying, &market)?;
    Ok(Response::new().add_attribute("action", "accept_market_upgrade"))
}

pub(super) fn cancel_upgrade(
    deps: DepsMut,
    info: MessageInfo,
    underlying: String,
) -> Result<Response, ContractError> {
    must_owner(&CONFIG.load(deps.storage)?, &info)?;
    let underlying = deps.api.addr_validate(&underlying)?;
    let mut market = load_market(deps.as_ref(), &underlying)?;
    market.pending_upgrade = None;
    MARKETS.save(deps.storage, &underlying, &market)?;
    Ok(Response::new().add_attribute("action", "cancel_market_upgrade"))
}

pub(super) fn accept_owner(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if config.pending_owner.as_ref() != Some(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }
    config.owner = info.sender;
    config.pending_owner = None;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "accept_owner"))
}

pub(super) fn cancel_owner(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    must_owner(&config, &info)?;
    config.pending_owner = None;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "cancel_owner_transfer"))
}

pub(super) fn migrate_markets(
    deps: DepsMut,
    env: Env,
    msg: MigrateMsg,
) -> Result<Response, ContractError> {
    let old = cw2::get_contract_version(deps.storage)?;
    let source =
        semver::Version::parse(&old.version).map_err(|_| ContractError::InvalidMigration {})?;
    if old.contract != CONTRACT_NAME
        || !["1.0.0", CONTRACT_VERSION]
            .iter()
            .any(|v| semver::Version::parse(v).ok().as_ref() == Some(&source))
    {
        return Err(ContractError::InvalidMigration {});
    }
    let config = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(config.owner.as_str())?;
    if let Some(owner) = config.pending_owner {
        deps.api.addr_validate(owner.as_str())?;
    }
    if config.debt_code_id == Some(0) {
        return Err(ContractError::InvalidMigration {});
    }
    let mut pins = std::collections::BTreeMap::new();
    for entry in msg.legacy_pins {
        let addr = deps.api.addr_validate(&entry.underlying)?;
        if pins.insert(addr, entry.pins).is_some() {
            return Err(ContractError::InvalidMigration {});
        }
    }
    let existing = MARKETS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    let mut verified = vec![];
    let mut receipts = std::collections::BTreeSet::new();
    for (addr, mut market) in existing {
        if addr != market.underlying
            || market.underlying == market.debt_token
            || market.underlying == env.contract.address
            || market.debt_token == env.contract.address
            || !receipts.insert(market.debt_token.clone())
            || !DEBT_TOKENS.has(deps.storage, &market.debt_token)
        {
            return Err(ContractError::InvalidMigration {});
        }
        if market.pins.is_none() {
            market.pins = Some(
                pins.remove(&addr)
                    .ok_or(ContractError::UnapprovedImplementation {})?,
            );
            // Receipt cw2 is read from storage, not a spoofable smart query. Code pins are supplied explicitly.
            let raw = deps
                .querier
                .query_wasm_raw(&market.debt_token, b"contract_info")?
                .ok_or(ContractError::InvalidMigration {})?;
            let debt_version: cw2::ContractVersion = from_json(raw)?;
            if debt_version.contract != "bridge-debt-token"
                || !["1.0.0", "1.1.0"].contains(&debt_version.version.as_str())
            {
                return Err(ContractError::InvalidMigration {});
            }
        }
        deps.api.addr_validate(market.debtor.as_str())?;
        if market.debtor == env.contract.address || market.debtor == market.debt_token {
            return Err(ContractError::InvalidDebtor {});
        }
        if let Some(limits) = &market.limits {
            if limits.epoch_seconds == 0
                || limits.max_per_issue.is_zero()
                || limits.max_outstanding.is_zero()
                || limits.max_per_epoch.is_zero()
                || market.epoch > env.block.time.seconds() / limits.epoch_seconds
            {
                return Err(ContractError::InvalidLimits {});
            }
        }
        assert_market_consistent(deps.as_ref(), &env, &market)?;
        verified.push((addr, market));
    }
    let indexed_receipts = DEBT_TOKENS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<std::collections::BTreeSet<_>>>()?;
    if !pins.is_empty() || indexed_receipts != receipts {
        return Err(ContractError::InvalidMigration {});
    }
    // Complete validation before writing, including when directly invoked in unit harnesses.
    for (addr, market) in verified {
        MARKETS.save(deps.storage, &addr, &market)?;
    }
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("action", "migrate"))
}
