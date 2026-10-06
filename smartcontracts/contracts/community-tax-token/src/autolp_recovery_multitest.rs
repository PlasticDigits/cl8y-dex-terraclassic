//! #1429: manager-approved recovery of a legacy, token-owned AutoLP sister.
//! The economic pair is synthetic; token execution and both AutoLP replies are real.
use super::*;
use cl8y_community_tax_autolp::{contract as sister, msg as sister_msg, state as sister_state};
use cosmwasm_std::{Decimal, StdError, WasmMsg};
use cw_storage_plus::Item;
use dex_common::autolp::LpPolicy;

const FIXTURE: &str = "fixture";
const POOL: Item<PoolResponse> = Item::new("test_pool");
const PAIR_INFO: Item<PairInfo> = Item::new("test_pair_info");
const ISSUANCE: Item<u16> = Item::new("test_issuance_bps");
const FAIL_CALLBACK: Item<bool> = Item::new("test_fail_callback");
const INVALID_POLICY_QUERY: Item<u8> = Item::new("test_invalid_policy_query");

#[cosmwasm_schema::cw_serde]
#[serde(untagged)]
enum SisterExecute {
    Fixture { test_state: SisterFixture },
    Production(sister_msg::ExecuteMsg),
}

#[cosmwasm_schema::cw_serde]
#[derive(Default)]
struct SisterFixture {
    manager: Option<Addr>,
    token: Option<Addr>,
    factory: Option<Addr>,
    quote: Option<Addr>,
    skimming: Option<bool>,
    fail_callback: Option<bool>,
    invalid_policy_query: Option<u8>,
}

fn legacy_sister_contract() -> Box<dyn Contract<Empty>> {
    fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: sister_msg::InstantiateMsg,
    ) -> Result<Response, cl8y_community_tax_autolp::error::ContractError> {
        let mut deps = deps;
        let response = sister::instantiate(deps.branch(), env, info, msg)?;
        // Migration from pre-policy wasm preserves Config and has no LP anchor.
        sister_state::LP_POLICY.remove(deps.storage);
        FAIL_CALLBACK.save(deps.storage, &false)?;
        Ok(response)
    }
    fn execute(
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: SisterExecute,
    ) -> Result<Response, cl8y_community_tax_autolp::error::ContractError> {
        match msg {
            SisterExecute::Fixture { test_state } => {
                assert_eq!(info.sender, Addr::unchecked(FIXTURE));
                let mut config = sister_state::CONFIG.load(deps.storage)?;
                if let Some(manager) = test_state.manager {
                    config.manager = manager;
                }
                if let Some(token) = test_state.token {
                    config.token = token;
                }
                if let Some(factory) = test_state.factory {
                    config.factory = factory;
                }
                if let Some(quote) = test_state.quote {
                    config.quote_token = Some(quote);
                }
                sister_state::CONFIG.save(deps.storage, &config)?;
                if let Some(lock) = test_state.skimming {
                    sister_state::SKIMMING.save(deps.storage, &lock)?;
                }
                if let Some(fail) = test_state.fail_callback {
                    FAIL_CALLBACK.save(deps.storage, &fail)?;
                }
                if let Some(mode) = test_state.invalid_policy_query {
                    INVALID_POLICY_QUERY.save(deps.storage, &mode)?;
                }
                Ok(Response::new())
            }
            SisterExecute::Production(msg) => {
                let update = matches!(msg, sister_msg::ExecuteMsg::UpdateConfig { .. });
                let response = sister::execute(deps.branch(), env, info, msg)?;
                if update && FAIL_CALLBACK.load(deps.storage)? {
                    // Failure after actual UpdateConfig persistence and registration.
                    return Ok(response.add_message(WasmMsg::Execute {
                        contract_addr: "missing_callback".into(),
                        msg: to_json_binary(&Empty {})?,
                        funds: vec![],
                    }));
                }
                Ok(response)
            }
        }
    }
    fn query(deps: Deps, env: Env, msg: sister_msg::QueryMsg) -> StdResult<Binary> {
        if matches!(msg, sister_msg::QueryMsg::GetLpPolicy {}) {
            match INVALID_POLICY_QUERY.may_load(deps.storage)?.unwrap_or(0) {
                1 => return Err(StdError::generic_err("legacy policy query unsupported")),
                2 => return Ok(Binary::from(b"malformed policy".as_slice())),
                _ => {}
            }
        }
        sister::query(deps, env, msg)
    }
    Box::new(ContractWrapper::new(execute, instantiate, query).with_reply(sister::reply))
}

#[cosmwasm_schema::cw_serde]
enum EconomicPairExecute {
    Receive(cw20::Cw20ReceiveMsg),
    ProvideLiquidity {
        assets: [Asset; 2],
        slippage_tolerance: Option<Decimal>,
        receiver: Option<String>,
        deadline: Option<u64>,
    },
    Fixture {
        pool: Option<PoolResponse>,
        lp_token: Option<Addr>,
        issuance_bps: Option<u16>,
    },
}

fn economic_pair_contract() -> Box<dyn Contract<Empty>> {
    fn instantiate(
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        assets: [AssetInfo; 2],
    ) -> StdResult<Response> {
        PAIR_INFO.save(
            deps.storage,
            &PairInfo {
                asset_infos: assets.clone(),
                contract_addr: env.contract.address,
                liquidity_token: Addr::unchecked("pending_lp"),
            },
        )?;
        POOL.save(
            deps.storage,
            &PoolResponse {
                assets: assets.map(|info| Asset {
                    info,
                    amount: Uint128::new(GENESIS),
                }),
                total_share: Uint128::new(GENESIS),
            },
        )?;
        ISSUANCE.save(deps.storage, &10_000)?;
        Ok(Response::new())
    }
    fn token_message(asset: &AssetInfo, msg: Cw20ExecuteMsg) -> StdResult<WasmMsg> {
        let AssetInfo::Token { contract_addr } = asset else {
            return Err(StdError::generic_err("fixture requires CW20 assets"));
        };
        Ok(WasmMsg::Execute {
            contract_addr: contract_addr.clone(),
            msg: to_json_binary(&msg)?,
            funds: vec![],
        })
    }
    fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: EconomicPairExecute,
    ) -> StdResult<Response> {
        let mut pool = POOL.load(deps.storage)?;
        match msg {
            EconomicPairExecute::Fixture {
                pool,
                lp_token,
                issuance_bps,
            } => {
                assert_eq!(info.sender, Addr::unchecked(FIXTURE));
                if let Some(pool) = pool {
                    POOL.save(deps.storage, &pool)?;
                }
                if let Some(lp_token) = lp_token {
                    let mut pair = PAIR_INFO.load(deps.storage)?;
                    pair.liquidity_token = lp_token;
                    PAIR_INFO.save(deps.storage, &pair)?;
                }
                if let Some(bps) = issuance_bps {
                    ISSUANCE.save(deps.storage, &bps)?;
                }
                Ok(Response::new())
            }
            EconomicPairExecute::Receive(receive) => {
                let Cw20HookMsg::Swap {
                    max_spread,
                    min_return,
                    to,
                    ..
                } = cosmwasm_std::from_json(&receive.msg)?
                else {
                    return Err(StdError::generic_err("fixture requires swap"));
                };
                let minimum = min_return
                    .filter(|floor| !floor.is_zero())
                    .ok_or_else(|| StdError::generic_err("positive swap floor required"))?;
                assert!(max_spread.is_some_and(|spread| spread <= Decimal::percent(2)));
                // Deterministic 1:1 settlement; the saved pool exercises policy and
                // minted-share guards independently of actual pair pricing.
                if receive.amount < minimum {
                    return Err(StdError::generic_err("minimum return violated"));
                }
                deps.storage.set(b"last_hook", receive.msg.as_slice());
                pool.assets[0].amount += receive.amount;
                pool.assets[1].amount = pool.assets[1].amount.checked_sub(receive.amount)?;
                POOL.save(deps.storage, &pool)?;
                Ok(Response::new().add_message(token_message(
                    &pool.assets[1].info,
                    Cw20ExecuteMsg::Transfer {
                        recipient: to.unwrap_or(receive.sender),
                        amount: receive.amount,
                    },
                )?))
            }
            EconomicPairExecute::ProvideLiquidity {
                assets,
                slippage_tolerance,
                receiver,
                ..
            } => {
                let shares = [0, 1].map(|index| {
                    assets[index]
                        .amount
                        .multiply_ratio(pool.total_share, pool.assets[index].amount)
                });
                let minimum = shares[0].min(shares[1]);
                let maximum = shares[0].max(shares[1]);
                let tolerance = slippage_tolerance
                    .ok_or_else(|| StdError::generic_err("provide tolerance required"))?;
                if minimum < maximum * (Decimal::one() - tolerance) {
                    return Err(StdError::generic_err("LP ratio tolerance exceeded"));
                }
                let minted = minimum.multiply_ratio(ISSUANCE.load(deps.storage)?, 10_000u16);
                let mut response = Response::new();
                for (index, asset) in assets.iter().enumerate() {
                    pool.assets[index].amount += asset.amount;
                    response = response.add_message(token_message(
                        &asset.info,
                        Cw20ExecuteMsg::TransferFrom {
                            owner: info.sender.to_string(),
                            recipient: env.contract.address.to_string(),
                            amount: asset.amount,
                        },
                    )?);
                }
                pool.total_share += minted;
                POOL.save(deps.storage, &pool)?;
                let lp = PAIR_INFO.load(deps.storage)?.liquidity_token;
                Ok(response.add_message(WasmMsg::Execute {
                    contract_addr: lp.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Mint {
                        recipient: receiver.unwrap_or(info.sender.to_string()),
                        amount: minted,
                    })?,
                    funds: vec![],
                }))
            }
        }
    }
    fn query(deps: Deps, _env: Env, msg: PairQuery) -> StdResult<Binary> {
        match msg {
            PairQuery::Pair {} => to_json_binary(&PAIR_INFO.load(deps.storage)?),
            PairQuery::Pool {} => to_json_binary(&POOL.load(deps.storage)?),
            _ => Err(StdError::generic_err("unsupported fixture query")),
        }
    }
    Box::new(ContractWrapper::new(execute, instantiate, query))
}

struct Recovery {
    token: EnvTok,
    sister: Addr,
    quote: Addr,
    lp: Addr,
}

fn recovery() -> Recovery {
    let mut e = clean(vec![Sku::AutoV2Lp, Sku::VariableRates], None);
    let cw20 = e.app.store_code(cw20_base_contract());
    let quote = e
        .app
        .instantiate_contract(
            cw20,
            e.manager.clone(),
            &cw20_base::msg::InstantiateMsg {
                name: "Quote".into(),
                symbol: "QUOTE".into(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: e.manager.to_string(),
                    amount: Uint128::new(GENESIS),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "quote",
            None,
        )
        .unwrap();
    let pair_code = e.app.store_code(economic_pair_contract());
    e.pair = e
        .app
        .instantiate_contract(
            pair_code,
            e.manager.clone(),
            &[
                AssetInfo::Token {
                    contract_addr: e.token.to_string(),
                },
                AssetInfo::Token {
                    contract_addr: quote.to_string(),
                },
            ],
            &[],
            "economic-pair",
            None,
        )
        .unwrap();
    let lp = e
        .app
        .instantiate_contract(
            cw20,
            e.manager.clone(),
            &cw20_base::msg::InstantiateMsg {
                name: "Liquidity".into(),
                symbol: "LPTEST".into(),
                decimals: 6,
                initial_balances: vec![],
                mint: Some(cw20::MinterResponse {
                    minter: e.pair.to_string(),
                    cap: None,
                }),
                marketing: None,
            },
            &[],
            "lp",
            None,
        )
        .unwrap();
    e.app
        .execute_contract(
            Addr::unchecked(FIXTURE),
            e.pair.clone(),
            &EconomicPairExecute::Fixture {
                pool: None,
                lp_token: Some(lp.clone()),
                issuance_bps: None,
            },
            &[],
        )
        .unwrap();
    e.app
        .execute_contract(
            e.manager.clone(),
            e.factory.clone(),
            &MockFactoryExec::Set {
                pair: e.pair.to_string(),
                token: e.token.to_string(),
            },
            &[],
        )
        .unwrap();
    let sister_code = e.app.store_code(legacy_sister_contract());
    let autolp = e
        .app
        .instantiate_contract(
            sister_code,
            e.manager.clone(),
            &sister_msg::InstantiateMsg {
                token: e.token.to_string(),
                manager: e.token.to_string(),
                factory: e.factory.to_string(),
                router: None,
                pair: Some(e.pair.to_string()),
                quote_token: None,
                threshold: Uint128::new(1_000),
                lp_recipient: e.manager.to_string(),
                skim_max_spread: None,
                skim_min_return: None,
            },
            &[],
            "legacy-token-owned-autolp",
            None,
        )
        .unwrap();
    e.app
        .execute_contract(
            Addr::unchecked("launcher"),
            e.token.clone(),
            &ExecuteMsg::BindAutolp {
                autolp: autolp.to_string(),
            },
            &[],
        )
        .unwrap();
    e.app
        .execute_contract(
            e.manager.clone(),
            quote.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: e.pair.to_string(),
                amount: Uint128::new(10_000_000),
            },
            &[],
        )
        .unwrap();
    Recovery {
        token: e,
        sister: autolp,
        quote,
        lp,
    }
}

impl Recovery {
    fn approved(&self) -> Option<LpPolicy> {
        self.token
            .app
            .wrap()
            .query_wasm_smart(&self.sister, &sister_msg::QueryMsg::GetLpPolicy {})
            .unwrap()
    }

    fn pool(&self) -> PoolResponse {
        self.token
            .app
            .wrap()
            .query_wasm_smart(&self.token.pair, &PairQuery::Pool {})
            .unwrap()
    }

    fn expected(&self) -> LpPolicy {
        let pool = self.pool();
        LpPolicy {
            pair: self.token.pair.clone(),
            quote: self.quote.clone(),
            lp_token: self.lp.clone(),
            tax_reserve: pool.assets[0].amount,
            quote_reserve: pool.assets[1].amount,
            total_share: pool.total_share,
        }
    }

    fn refresh(&mut self) {
        let expected_policy = self.expected();
        self.token
            .app
            .execute_contract(
                self.token.manager.clone(),
                self.token.token.clone(),
                &ExecuteMsg::RefreshAutolpPolicy { expected_policy },
                &[],
            )
            .unwrap();
    }

    fn fixture(&mut self, test_state: SisterFixture) {
        self.token
            .app
            .execute_contract(
                Addr::unchecked(FIXTURE),
                self.sister.clone(),
                &SisterExecute::Fixture { test_state },
                &[],
            )
            .unwrap();
    }

    fn set_pool(&mut self, pool: PoolResponse) {
        self.token
            .app
            .execute_contract(
                Addr::unchecked(FIXTURE),
                self.token.pair.clone(),
                &EconomicPairExecute::Fixture {
                    pool: Some(pool),
                    lp_token: None,
                    issuance_bps: None,
                },
                &[],
            )
            .unwrap();
    }

    fn fund(&mut self) {
        self.token
            .app
            .execute_contract(
                self.token.manager.clone(),
                self.token.token.clone(),
                &ExecuteMsg::Transfer {
                    recipient: self.sister.to_string(),
                    amount: Uint128::new(2_000_000),
                },
                &[],
            )
            .unwrap();
    }

    fn skim(&mut self) -> Result<cw_multi_test::AppResponse, String> {
        self.token
            .app
            .execute_contract(
                Addr::unchecked("keeper"),
                self.sister.clone(),
                &sister_msg::ExecuteMsg::SkimToLp {},
                &[],
            )
            .map_err(|error| format!("{error:?}"))
    }
}

#[test]
fn recovery1429_manager_explicit_refresh_initializes_policy_and_permissionless_skim() {
    let mut h = recovery();
    assert_eq!(h.approved(), None);
    h.fund();
    let missing = h.skim().unwrap_err();
    assert!(format!("{missing:?}").contains("policy"), "{missing:?}");
    let before = autolp_cfg(&h.token, &h.sister);
    let expected = h.expected();
    h.refresh();
    assert_eq!(h.approved(), Some(expected));
    assert_eq!(autolp_cfg(&h.token, &h.sister), before);
    assert_eq!(
        balance(&h.token.app, &h.token.ust1, h.token.cmm.as_str()),
        0
    );
    h.skim().unwrap();
    let raw = h
        .token
        .app
        .wrap()
        .query_wasm_raw(&h.token.pair, b"last_hook".as_slice())
        .unwrap()
        .unwrap();
    let Cw20HookMsg::Swap { min_return, .. } = cosmwasm_std::from_json(raw).unwrap() else {
        panic!("skim must send a swap hook");
    };
    assert_eq!(min_return, Some(Uint128::new(990_000)));
    assert!(balance(&h.token.app, &h.lp, h.token.manager.as_str()) >= 990_000);
    assert!(!autolp_cfg(&h.token, &h.sister).skimming);
}

#[test]
fn recovery1429_foreign_callers_and_direct_human_sister_refresh_are_unauthorized() {
    let mut h = recovery();
    for caller in [
        h.token.user.clone(),
        Addr::unchecked("launcher"),
        Addr::unchecked("keeper"),
    ] {
        let expected_policy = h.expected();
        let error = h
            .token
            .app
            .execute_contract(
                caller,
                h.token.token.clone(),
                &ExecuteMsg::RefreshAutolpPolicy { expected_policy },
                &[],
            )
            .unwrap_err();
        assert!(format!("{error:?}").contains("Unauthorized"), "{error:?}");
        assert_eq!(h.approved(), None);
    }
    let error = h
        .token
        .app
        .execute_contract(
            h.token.manager.clone(),
            h.sister.clone(),
            &sister_msg::ExecuteMsg::UpdateConfig {
                pair: Some(h.token.pair.to_string()),
                router: None,
                quote_token: None,
                threshold: None,
                lp_recipient: None,
                skim_max_spread: None,
                skim_min_return: None,
            },
            &[],
        )
        .unwrap_err();
    assert!(format!("{error:?}").contains("Unauthorized"), "{error:?}");
    assert_eq!(h.approved(), None);
}

#[test]
fn recovery1429_rejects_foreign_sister_identity_and_active_skim_without_writes() {
    let cases = [
        SisterFixture {
            manager: Some(Addr::unchecked("human_manager")),
            ..Default::default()
        },
        SisterFixture {
            token: Some(Addr::unchecked("foreign_token")),
            ..Default::default()
        },
        SisterFixture {
            factory: Some(Addr::unchecked("foreign_factory")),
            ..Default::default()
        },
        SisterFixture {
            quote: Some(Addr::unchecked("foreign_quote")),
            ..Default::default()
        },
        SisterFixture {
            skimming: Some(true),
            ..Default::default()
        },
    ];
    for fixture in cases {
        let mut h = recovery();
        h.fixture(fixture);
        let before = autolp_cfg(&h.token, &h.sister);
        let expected_policy = h.expected();
        assert!(h
            .token
            .app
            .execute_contract(
                h.token.manager.clone(),
                h.token.token.clone(),
                &ExecuteMsg::RefreshAutolpPolicy { expected_policy },
                &[],
            )
            .is_err());
        assert_eq!(h.approved(), None);
        assert_eq!(autolp_cfg(&h.token, &h.sister), before);
    }
}

#[test]
fn recovery1429_rejects_snapshot_identity_zero_amounts_and_stale_live_pool() {
    for field in 0..8 {
        let mut h = recovery();
        let mut expected_policy = h.expected();
        match field {
            0 => expected_policy.pair = Addr::unchecked("foreign_pair"),
            1 => expected_policy.quote = Addr::unchecked("foreign_quote"),
            2 => expected_policy.lp_token = Addr::unchecked("foreign_lp"),
            3 => expected_policy.tax_reserve = Uint128::zero(),
            4 => expected_policy.quote_reserve = Uint128::zero(),
            5 => expected_policy.total_share = Uint128::zero(),
            6 => expected_policy.tax_reserve += Uint128::one(),
            _ => {
                let mut pool = h.pool();
                pool.total_share += Uint128::one();
                h.set_pool(pool);
            }
        }
        assert!(
            h.token
                .app
                .execute_contract(
                    h.token.manager.clone(),
                    h.token.token.clone(),
                    &ExecuteMsg::RefreshAutolpPolicy { expected_policy },
                    &[],
                )
                .is_err(),
            "accepted invalid snapshot case {field}"
        );
        assert_eq!(h.approved(), None);
    }
}

#[test]
fn recovery1429_unchanged_settings_do_not_create_policy_or_charge_invoice() {
    let mut h = recovery();
    let before = balance(&h.token.app, &h.token.ust1, h.token.manager.as_str());
    let batch = identical_autolp_batch(&h.token, 1_000, &h.token.manager);
    let error = h
        .token
        .app
        .execute_contract(
            h.token.manager.clone(),
            h.token.ust1.clone(),
            &Cw20ExecuteMsg::Send {
                contract: h.token.token.to_string(),
                amount: Uint128::new(INVOICE_UST1),
                msg: to_json_binary(&InvoiceHookMsg::UpdateSettings {
                    settings: SettingsBatch {
                        autolp: Some(batch),
                        ..Default::default()
                    },
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("empty or identical"),
        "{error:?}"
    );
    assert_eq!(h.approved(), None);
    assert_eq!(
        balance(&h.token.app, &h.token.ust1, h.token.manager.as_str()),
        before
    );
    assert_eq!(
        balance(&h.token.app, &h.token.ust1, h.token.cmm.as_str()),
        0
    );
    let batch = identical_autolp_batch(&h.token, 1_000, &h.token.manager);
    settings_batch(
        &mut h.token,
        SettingsBatch {
            buy_bps: Some(600),
            autolp: Some(batch),
            ..Default::default()
        },
    );
    assert_eq!(h.approved(), None);
    assert_eq!(
        balance(&h.token.app, &h.token.ust1, h.token.cmm.as_str()),
        INVOICE_UST1
    );
}

#[test]
fn recovery1429_downstream_refresh_failure_rolls_back_new_anchor_and_can_retry() {
    let mut h = recovery();
    h.refresh();
    let original = h.approved();
    let mut pool = h.pool();
    for asset in &mut pool.assets {
        asset.amount += Uint128::new(10_000_000);
    }
    h.set_pool(pool);
    h.fixture(SisterFixture {
        fail_callback: Some(true),
        ..Default::default()
    });
    let config = autolp_cfg(&h.token, &h.sister);
    let expected_policy = h.expected();
    let error = h
        .token
        .app
        .execute_contract(
            h.token.manager.clone(),
            h.token.token.clone(),
            &ExecuteMsg::RefreshAutolpPolicy { expected_policy },
            &[],
        )
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("missing_callback"),
        "{error:?}"
    );
    assert_eq!(h.approved(), original);
    assert_eq!(autolp_cfg(&h.token, &h.sister), config);
    h.fixture(SisterFixture {
        fail_callback: Some(false),
        ..Default::default()
    });
    h.refresh();
    assert_eq!(h.approved(), Some(h.expected()));
}

fn escrow_snapshot(h: &Recovery) -> Vec<Uint128> {
    let mut values = vec![];
    for token in [&h.token.token, &h.quote, &h.lp] {
        for owner in [
            &h.sister,
            &h.token.pair,
            &h.token.manager,
            &h.token.treasury,
            &h.token.cmm,
        ] {
            values.push(Uint128::new(balance(&h.token.app, token, owner.as_str())));
        }
    }
    for token in [&h.token.token, &h.quote] {
        let allowance: cw20::AllowanceResponse = h
            .token
            .app
            .wrap()
            .query_wasm_smart(
                token,
                &Cw20QueryMsg::Allowance {
                    owner: h.sister.to_string(),
                    spender: h.token.pair.to_string(),
                },
            )
            .unwrap();
        values.push(allowance.allowance);
    }
    let supply: cw20::TokenInfoResponse = h
        .token
        .app
        .wrap()
        .query_wasm_smart(&h.lp, &Cw20QueryMsg::TokenInfo {})
        .unwrap();
    values.push(supply.total_supply);
    values
}

#[test]
fn recovery1429_insufficient_lp_reply_reverts_full_skim_and_allows_corrected_retry() {
    let mut h = recovery();
    h.refresh();
    h.fund();
    h.token
        .app
        .execute_contract(
            Addr::unchecked(FIXTURE),
            h.token.pair.clone(),
            &EconomicPairExecute::Fixture {
                pool: None,
                lp_token: None,
                issuance_bps: Some(9_000),
            },
            &[],
        )
        .unwrap();
    let before = escrow_snapshot(&h);
    let pool = h.pool();
    let policy = h.approved();
    let error = h.skim().unwrap_err();
    assert!(
        format!("{error:?}").contains("LP issuance below approved minimum"),
        "{error:?}"
    );
    assert_eq!(escrow_snapshot(&h), before);
    assert_eq!(h.pool(), pool);
    assert_eq!(h.approved(), policy);
    assert!(!autolp_cfg(&h.token, &h.sister).skimming);
    assert!(h
        .token
        .app
        .wrap()
        .query_wasm_raw(&h.sister, b"pending_skim".as_slice())
        .unwrap()
        .is_none());
    assert!(h
        .token
        .app
        .wrap()
        .query_wasm_raw(&h.token.pair, b"last_hook".as_slice())
        .unwrap()
        .is_none());
    h.token
        .app
        .execute_contract(
            Addr::unchecked(FIXTURE),
            h.token.pair.clone(),
            &EconomicPairExecute::Fixture {
                pool: None,
                lp_token: None,
                issuance_bps: Some(10_000),
            },
            &[],
        )
        .unwrap();
    h.skim().unwrap();
}

#[test]
fn recovery1429_stale_policy_keeper_cannot_refresh_and_ratio_failure_is_atomic() {
    let mut h = recovery();
    h.refresh();
    h.fund();
    let mut pool = h.pool();
    for asset in &mut pool.assets {
        asset.amount += Uint128::new(20_000_000);
    }
    h.set_pool(pool);
    let approved = h.approved();
    let before = escrow_snapshot(&h);
    let error = h.skim().unwrap_err();
    assert!(
        format!("{error:?}").contains("approved policy tolerance"),
        "{error:?}"
    );
    assert_eq!(h.approved(), approved);
    assert_eq!(escrow_snapshot(&h), before);
    h.refresh();
    h.token
        .app
        .execute_contract(
            h.token.manager.clone(),
            h.quote.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: h.sister.to_string(),
                amount: Uint128::new(1_000_000),
            },
            &[],
        )
        .unwrap();
    let before = escrow_snapshot(&h);
    let pool = h.pool();
    let error = h.skim().unwrap_err();
    assert!(
        format!("{error:?}").contains("LP ratio tolerance exceeded"),
        "{error:?}"
    );
    assert_eq!(escrow_snapshot(&h), before);
    assert_eq!(h.pool(), pool);
    assert!(!autolp_cfg(&h.token, &h.sister).skimming);
}

#[test]
fn recovery1429_missing_binding_and_disabled_sku_reject_before_sister_query() {
    for features in [vec![], vec![Sku::AutoV2Lp]] {
        let mut e = clean(features, None);
        let expected_policy = LpPolicy {
            pair: e.pair.clone(),
            quote: Addr::unchecked("quote_token"),
            lp_token: Addr::unchecked("mock_lp_token"),
            tax_reserve: Uint128::new(GENESIS),
            quote_reserve: Uint128::new(GENESIS),
            total_share: Uint128::new(GENESIS),
        };
        let error = e
            .app
            .execute_contract(
                e.manager.clone(),
                e.token.clone(),
                &ExecuteMsg::RefreshAutolpPolicy { expected_policy },
                &[],
            )
            .unwrap_err();
        let text = format!("{error:?}");
        assert!(
            text.contains("not unlocked") || text.contains("AutoLP"),
            "{text}"
        );
        assert!(!text.contains("No such contract"), "{text}");
    }
}

#[test]
fn recovery1429_snapshot_message_roundtrip_and_token_migration_preserve_state() {
    let mut h = recovery();
    h.refresh();
    h.fund();
    let message = ExecuteMsg::RefreshAutolpPolicy {
        expected_policy: h.expected(),
    };
    let encoded = to_json_binary(&message).unwrap();
    let decoded: ExecuteMsg = cosmwasm_std::from_json(&encoded).unwrap();
    assert_eq!(decoded, message);
    assert!(String::from_utf8(encoded.to_vec())
        .unwrap()
        .contains("refresh_autolp_policy"));

    let before = escrow_snapshot(&h);
    let config: ConfigResponse = h
        .token
        .app
        .wrap()
        .query_wasm_smart(&h.token.token, &QueryMsg::GetConfig {})
        .unwrap();
    let policy = h.approved();
    // Recreate the reviewed predecessor cw2 stamp without altering balances,
    // binding, manager, feature flags or router state.
    let legacy_code = h.token.app.store_code(Box::new(
        ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_migrate(
            |deps: DepsMut, _env: Env, _msg: Empty| -> StdResult<Response> {
                cw2::set_contract_version(
                    deps.storage,
                    "crates.io:cl8y-community-tax-token",
                    "1.3.0",
                )?;
                Ok(Response::new())
            },
        ),
    ));
    h.token
        .app
        .migrate_contract(
            Addr::unchecked("cmm_gov"),
            h.token.token.clone(),
            &Empty {},
            legacy_code,
        )
        .unwrap();
    let current_code = h.token.app.store_code(Box::new(
        ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_migrate(crate::contract::migrate),
    ));
    h.token
        .app
        .migrate_contract(
            Addr::unchecked("cmm_gov"),
            h.token.token.clone(),
            &crate::msg::MigrateMsg { adopt: None },
            current_code,
        )
        .unwrap();
    let after_config: ConfigResponse = h
        .token
        .app
        .wrap()
        .query_wasm_smart(&h.token.token, &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(after_config, config);
    assert_eq!(escrow_snapshot(&h), before);
    assert_eq!(h.approved(), policy);
    h.refresh();
}

#[test]
fn recovery1429_unsupported_or_malformed_policy_query_fails_closed() {
    for mode in [1, 2] {
        let mut h = recovery();
        h.fixture(SisterFixture {
            invalid_policy_query: Some(mode),
            ..Default::default()
        });
        let expected_policy = h.expected();
        let config = autolp_cfg(&h.token, &h.sister);
        assert!(h
            .token
            .app
            .execute_contract(
                h.token.manager.clone(),
                h.token.token.clone(),
                &ExecuteMsg::RefreshAutolpPolicy { expected_policy },
                &[],
            )
            .is_err());
        assert_eq!(autolp_cfg(&h.token, &h.sister), config);
        // An unsupported query is not treated as the supported null legacy anchor.
        h.fixture(SisterFixture {
            invalid_policy_query: Some(0),
            ..Default::default()
        });
        assert_eq!(h.approved(), None);
        h.refresh();
        assert_eq!(h.approved(), Some(h.expected()));
    }
}

#[test]
fn recovery1429_attached_native_funds_rejected_and_refunded_atomically() {
    let mut h = recovery();
    h.token
        .app
        .init_modules(|router, _api, storage| {
            router
                .bank
                .init_balance(storage, &h.token.manager, cosmwasm_std::coins(100, "uluna"))
        })
        .unwrap();
    let expected_policy = h.expected();
    assert!(h
        .token
        .app
        .execute_contract(
            h.token.manager.clone(),
            h.token.token.clone(),
            &ExecuteMsg::RefreshAutolpPolicy { expected_policy },
            &cosmwasm_std::coins(100, "uluna"),
        )
        .is_err());
    assert_eq!(h.approved(), None);
    assert_eq!(
        h.token
            .app
            .wrap()
            .query_balance(&h.token.manager, "uluna")
            .unwrap()
            .amount,
        Uint128::new(100)
    );
    assert_eq!(
        h.token
            .app
            .wrap()
            .query_balance(&h.token.token, "uluna")
            .unwrap()
            .amount,
        Uint128::zero()
    );
}

#[test]
fn recovery1429_factory_delisting_rejects_actual_sister_rebind_atomically() {
    let mut h = recovery();
    let expected_policy = h.expected();
    let before = autolp_cfg(&h.token, &h.sister);
    h.token
        .app
        .execute_contract(
            h.token.manager.clone(),
            h.token.factory.clone(),
            &MockFactoryExec::Set {
                pair: "different_listed_pair".into(),
                token: h.token.token.to_string(),
            },
            &[],
        )
        .unwrap();
    let error = h
        .token
        .app
        .execute_contract(
            h.token.manager.clone(),
            h.token.token.clone(),
            &ExecuteMsg::RefreshAutolpPolicy { expected_policy },
            &[],
        )
        .unwrap_err();
    assert!(format!("{error:?}").contains("factory-listed"), "{error:?}");
    assert_eq!(h.approved(), None);
    assert_eq!(autolp_cfg(&h.token, &h.sister), before);
}

#[test]
fn recovery1429_threshold_setting_delta_cannot_silently_approve_missing_policy() {
    let mut h = recovery();
    let mut batch = identical_autolp_batch(&h.token, 1_000, &h.token.manager);
    batch.threshold = Uint128::new(2_000);
    settings_batch(
        &mut h.token,
        SettingsBatch {
            autolp: Some(batch),
            ..Default::default()
        },
    );
    assert_eq!(
        autolp_cfg(&h.token, &h.sister).threshold,
        Uint128::new(2_000)
    );
    assert_eq!(h.approved(), None);
    assert_eq!(
        balance(&h.token.app, &h.token.ust1, h.token.cmm.as_str()),
        INVOICE_UST1
    );
}
