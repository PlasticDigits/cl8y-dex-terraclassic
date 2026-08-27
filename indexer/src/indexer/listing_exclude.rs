//! Shared listing-crawler exclude list (GitLab #685 / **L639-2** / #562).
//!
//! `/cg/*`, `/cmc/*`, and `/gt/*` omit the same CW20s: columbus-5 gems plus
//! ALPHA / USTRIX / SpaceUSD. Identity is **contract address**, not ticker.
//! Keep this list lockstep with [`super::defillama::COLUMBUS5_GEM_ADDRESSES`].

use std::collections::{HashMap, HashSet};

use super::defillama::COLUMBUS5_GEM_ADDRESSES;
use crate::config::{
    DEFAULT_HUB_CL8Y_ADDRESS, DEFAULT_HUB_CLUNC_ADDRESS, DEFAULT_HUB_CUSTC_ADDRESS,
    DEFAULT_HUB_UST1_ADDRESS, DEFAULT_HUB_USTR_ADDRESS,
};

/// ALPHA / USTRIX / SpaceUSD — extra pins beyond `#562` gems (same as `/gt` `EXCLUDED_CW20` tail).
pub const LISTING_EXTRA_EXCLUDED_CW20: &[&str] = &[
    "terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz", // ALPHA
    "terra1r3eaa2tucjr3es88wzuqpgxvssqflk9cghrjmf9uneds8wljyapqwtrcp5", // USTRIX
    "terra1cvd5cgrs8rrl96hte34n57497u5f9cwuv3e6ztxgetkx4uzmcdyswv79zl", // SpaceUSD
];

/// Full `/gt` exclude list (gems + extras) for lockstep tests and SQL `<> ALL`.
pub const LISTING_EXCLUDED_CW20: &[&str] = &[
    "terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94",
    "terra1k6cqupylk0wp4pj273pntwhv9py0q5guyqye8ssvukn9xq7mes7sdlmena",
    "terra1ejq3mjjgnklpa3pg4jterlfwsny055gpmcjf3fz0ev3ueajnzeysz6xxgr",
    "terra178fgrfzv7njtmdp9vghyf2dx77sah8u8jluzs7ym562chaxnmj2s6mn6m9",
    "terra1fga508hzx8dd7x8q4uhm6mdhkqv6fxrtsea3r27smdqmv5k2jgxq5zk9fc",
    "terra12k67cvfs7y7g8lca3qr4g4py6s6j69fu24gze5pjfamfpckv8mps7cymme",
    "terra17dpnjlpgsnm8muu4msfjra4f2hrptnjp2jdpkka4p0e3px42ayxq0pmc2z",
    "terra18fzufz8cs7ez49xjwgs248x85za5v50yug55fj7lyxp9hapxyr7qnh3czs",
    "terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz",
    "terra1r3eaa2tucjr3es88wzuqpgxvssqflk9cghrjmf9uneds8wljyapqwtrcp5",
    "terra1cvd5cgrs8rrl96hte34n57497u5f9cwuv3e6ztxgetkx4uzmcdyswv79zl",
];

fn addr_eq(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// True when `addr` is a gem, ALPHA, USTRIX, or SpaceUSD CW20.
pub fn is_excluded_cw20(addr: &str) -> bool {
    let lower = addr.trim().to_ascii_lowercase();
    LISTING_EXCLUDED_CW20.iter().any(|a| *a == lower.as_str())
        || COLUMBUS5_GEM_ADDRESSES.iter().any(|a| *a == lower.as_str())
        || LISTING_EXTRA_EXCLUDED_CW20
            .iter()
            .any(|a| *a == lower.as_str())
}

/// Lowercased bind list for SQL `<> ALL($n)` (gems + extras, de-duped).
pub fn listing_excluded_cw20_binds() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for addr in LISTING_EXCLUDED_CW20
        .iter()
        .chain(COLUMBUS5_GEM_ADDRESSES.iter())
        .chain(LISTING_EXTRA_EXCLUDED_CW20.iter())
    {
        let lower = addr.to_ascii_lowercase();
        if !out.iter().any(|e| e == &lower) {
            out.push(lower);
        }
    }
    out
}

/// Pair is omitted from CG/CMC/GT when either leg is an excluded CW20.
pub fn pair_is_listing_excluded(a0_contract: Option<&str>, a1_contract: Option<&str>) -> bool {
    [a0_contract, a1_contract]
        .into_iter()
        .flatten()
        .any(is_excluded_cw20)
}

/// Permanent economic pins (natives + hub six). Used when `/cmc/assets` symbols collide.
pub fn is_listing_economic_asset(contract: Option<&str>, denom: Option<&str>) -> bool {
    if let Some(d) = denom {
        if d == "uluna" || d == "uusd" {
            return true;
        }
    }
    contract.is_some_and(|c| {
        addr_eq(c, DEFAULT_HUB_CL8Y_ADDRESS)
            || addr_eq(c, DEFAULT_HUB_CLUNC_ADDRESS)
            || addr_eq(c, DEFAULT_HUB_CUSTC_ADDRESS)
            || addr_eq(c, DEFAULT_HUB_UST1_ADDRESS)
            || addr_eq(c, DEFAULT_HUB_USTR_ADDRESS)
    })
}

/// Insert `ticker_id → pool_id`. Duplicate keys (two pairs, same symbols) are **dropped**
/// so orderbook/trades cannot silently bind the first insert (GitLab #685 **AC10**).
pub fn record_unique_ticker(
    map: &mut HashMap<String, String>,
    collisions: &mut HashSet<String>,
    ticker_id: String,
    pool_id: String,
) {
    if collisions.contains(&ticker_id) {
        return;
    }
    match map.get(&ticker_id) {
        Some(existing) if existing != &pool_id => {
            map.remove(&ticker_id);
            collisions.insert(ticker_id);
        }
        Some(_) => {}
        None => {
            map.insert(ticker_id, pool_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gems_and_extras_excluded() {
        assert!(is_excluded_cw20(COLUMBUS5_GEM_ADDRESSES[0]));
        assert!(is_excluded_cw20(LISTING_EXTRA_EXCLUDED_CW20[0]));
        assert!(is_excluded_cw20(LISTING_EXTRA_EXCLUDED_CW20[2]));
        assert!(!is_excluded_cw20(DEFAULT_HUB_CL8Y_ADDRESS));
    }

    #[test]
    fn gem_const_lockstep() {
        for addr in COLUMBUS5_GEM_ADDRESSES {
            assert!(is_excluded_cw20(addr), "{addr}");
            assert!(
                LISTING_EXCLUDED_CW20.iter().any(|a| *a == *addr),
                "LISTING_EXCLUDED_CW20 missing gem {addr}"
            );
        }
        for addr in LISTING_EXTRA_EXCLUDED_CW20 {
            assert!(is_excluded_cw20(addr), "{addr}");
        }
    }

    #[test]
    fn unique_ticker_drops_collisions() {
        let mut map = HashMap::new();
        let mut collisions = HashSet::new();
        record_unique_ticker(
            &mut map,
            &mut collisions,
            "UST1_USTR".into(),
            "terra1a".into(),
        );
        record_unique_ticker(
            &mut map,
            &mut collisions,
            "UST1_USTR".into(),
            "terra1b".into(),
        );
        assert!(map.get("UST1_USTR").is_none());
        assert!(collisions.contains("UST1_USTR"));
        record_unique_ticker(
            &mut map,
            &mut collisions,
            "UST1_USTR".into(),
            "terra1c".into(),
        );
        assert!(map.get("UST1_USTR").is_none());
    }

    #[test]
    fn economic_pins_are_natives_and_hub() {
        assert!(is_listing_economic_asset(None, Some("uluna")));
        assert!(is_listing_economic_asset(
            Some(DEFAULT_HUB_CL8Y_ADDRESS),
            None
        ));
        assert!(!is_listing_economic_asset(
            Some(COLUMBUS5_GEM_ADDRESSES[0]),
            None
        ));
    }
}
