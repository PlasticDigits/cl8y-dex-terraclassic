use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{from_json, Addr, QuerierWrapper};

/// cw2 `contract_info` JSON shape, read via raw storage to learn a factory's version
/// without depending on the cw2 crate. Mirrors `cw2::ContractVersion`.
#[cw_serde]
struct Cw2ContractVersion {
    contract: String,
    version: String,
}

/// Decide whether a factory `BlacklistCheck` query *error* should BLOCK the trade
/// (GitLab #456 / SEC-I03 F02).
///
/// The pair/router blacklist guards historically failed OPEN on any factory query error,
/// for compatibility with pre-1.5.0 factories (no `BlacklistCheck` handler). That silently
/// disabled all blacklist enforcement if the stored factory address ever went stale — e.g.
/// an emergency factory migration that didn't update the pairs' stored factory pointer.
///
/// We now version-gate the silent pass by reading the factory's cw2 contract version from
/// raw storage (`contract_info`):
///   - factory reachable AND version >= 1.5.0 -> `BlacklistCheck` must work; an error is
///     anomalous (factory present but its check failed/incompatible) -> **block** (`true`)
///   - factory reachable AND version <  1.5.0 -> documented pre-1.5.0 compatibility ->
///     allow the legacy fail-open (`false`)
///   - factory version unreadable (no contract at the address, or not a cw2 contract)
///     -> preserve the legacy fail-open (`false`) so genuine pre-1.5.0 factories and unit-test
///     doubles keep working.
///
/// NOTE (residual): an orphaned/typo'd factory address that resolves to no contract still fails
/// open here. On Terra Classic a stored factory pointer effectively can never resolve to "no
/// contract" (contracts are not deletable, so a bad address is only a deploy-time typo), so this
/// residual is near-theoretical. If a stricter posture is wanted, flip the `None` arm to `true`
/// and update the tests.
///
/// The version probe only runs on the error path, so the normal (BlacklistCheck succeeds)
/// flow pays no extra gas.
pub fn blacklist_query_error_blocks(querier: &QuerierWrapper, factory: &Addr) -> bool {
    match factory_cw2_minor_version(querier, factory) {
        Some((major, minor)) => (major, minor) >= (1, 5),
        None => false,
    }
}

/// Read the factory's cw2 contract version as `(major, minor)` from raw storage.
/// cw2 stores `ContractVersion` JSON under the item key `contract_info`.
fn factory_cw2_minor_version(querier: &QuerierWrapper, factory: &Addr) -> Option<(u64, u64)> {
    let raw = querier
        .query_wasm_raw(factory.to_string(), b"contract_info".to_vec())
        .ok()
        .flatten()?;
    let parsed: Cw2ContractVersion = from_json(&raw).ok()?;
    let mut parts = parsed.version.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    Some((major, minor))
}

/// Combined blacklist probe for pair/router guards and dApp UX (GitLab #308).
#[cw_serde]
pub struct BlacklistCheck {
    pub wallet: Option<String>,
    pub tokens: Vec<String>,
    /// Single pair probe (legacy / pair contract guard).
    #[serde(default)]
    pub pair: Option<String>,
    /// Multihop router: any listed pair may trigger `blocked`.
    #[serde(default)]
    pub pairs: Vec<String>,
}

#[cw_serde]
pub struct BlacklistCheckResponse {
    pub blocked: bool,
    pub wallet_blacklisted: bool,
    pub blacklisted_tokens: Vec<Addr>,
    pub pair_blacklisted: bool,
    pub blacklisted_pairs: Vec<Addr>,
}

/// Paginated wallet blacklist for indexer / dashboards.
#[cw_serde]
#[derive(QueryResponses)]
pub enum BlacklistQueryMsg {
    #[returns(BlacklistCheckResponse)]
    BlacklistCheck(BlacklistCheck),
    #[returns(BlacklistedWalletsResponse)]
    BlacklistedWallets {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(BlacklistedTokensResponse)]
    BlacklistedTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(BlacklistedPairsResponse)]
    BlacklistedPairs {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct BlacklistedWalletsResponse {
    pub wallets: Vec<Addr>,
    pub next: Option<String>,
}

#[cw_serde]
pub struct BlacklistedTokensResponse {
    pub tokens: Vec<Addr>,
    pub next: Option<String>,
}

#[cw_serde]
pub struct BlacklistedPairsResponse {
    pub pairs: Vec<Addr>,
    pub next: Option<String>,
}

#[cfg(test)]
mod blacklist_query_error_tests {
    use super::*;
    use cosmwasm_std::testing::MockQuerier;
    use cosmwasm_std::{to_json_vec, ContractResult, QuerierWrapper, SystemResult, WasmQuery};

    /// Build a querier whose raw `contract_info` lookup at `factory` returns the given cw2
    /// version JSON, and errors (no contract) for any other address.
    fn querier_with_factory_version(
        factory: &str,
        version: Option<&str>,
    ) -> MockQuerier {
        let factory = factory.to_string();
        let version = version.map(|v| v.to_string());
        let mut q = MockQuerier::default();
        q.update_wasm(move |req| match req {
            WasmQuery::Raw { contract_addr, key } if *contract_addr == factory => {
                if key.as_slice() == b"contract_info" {
                    match &version {
                        Some(v) => {
                            let cv = Cw2ContractVersion {
                                contract: "cl8y-dex-factory".to_string(),
                                version: v.clone(),
                            };
                            SystemResult::Ok(ContractResult::Ok(to_json_vec(&cv).unwrap().into()))
                        }
                        None => SystemResult::Ok(ContractResult::Ok(
                            cosmwasm_std::Binary::default(),
                        )),
                    }
                } else {
                    SystemResult::Ok(ContractResult::Ok(cosmwasm_std::Binary::default()))
                }
            }
            // Any other contract address: no contract at that address.
            WasmQuery::Raw { contract_addr, .. } => SystemResult::Err(
                cosmwasm_std::SystemError::NoSuchContract {
                    addr: contract_addr.clone(),
                },
            ),
            _ => SystemResult::Err(cosmwasm_std::SystemError::Unknown {}),
        });
        q
    }

    #[test]
    fn current_factory_error_blocks() {
        let q = querier_with_factory_version("factory", Some("1.5.0"));
        let wrapper = QuerierWrapper::new(&q);
        assert!(blacklist_query_error_blocks(
            &wrapper,
            &Addr::unchecked("factory")
        ));
    }

    #[test]
    fn newer_factory_error_blocks() {
        let q = querier_with_factory_version("factory", Some("2.0.0"));
        let wrapper = QuerierWrapper::new(&q);
        assert!(blacklist_query_error_blocks(
            &wrapper,
            &Addr::unchecked("factory")
        ));
    }

    #[test]
    fn pre_1_5_factory_error_fails_open() {
        let q = querier_with_factory_version("factory", Some("1.4.9"));
        let wrapper = QuerierWrapper::new(&q);
        assert!(!blacklist_query_error_blocks(
            &wrapper,
            &Addr::unchecked("factory")
        ));
    }

    #[test]
    fn stale_address_with_no_contract_fails_open() {
        let q = querier_with_factory_version("factory", Some("1.5.0"));
        let wrapper = QuerierWrapper::new(&q);
        // Guard configured with a different (stale) factory address that has no contract:
        // version unreadable -> preserve the legacy fail-open (documented residual).
        assert!(!blacklist_query_error_blocks(
            &wrapper,
            &Addr::unchecked("stale_factory")
        ));
    }

    #[test]
    fn factory_without_cw2_info_fails_open() {
        // Contract exists at the address but has no parseable cw2 contract_info (e.g. a
        // unit-test double) -> version unreadable -> preserve the legacy fail-open.
        let q = querier_with_factory_version("factory", None);
        let wrapper = QuerierWrapper::new(&q);
        assert!(!blacklist_query_error_blocks(
            &wrapper,
            &Addr::unchecked("factory")
        ));
    }
}
