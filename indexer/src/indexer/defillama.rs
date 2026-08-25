//! DeFiLlama UTC-day helpers (GitLab #631).
//!
//! Volume/fees GET reads a materialized rollup. TVL is **on-chain only** (factory
//! `Pairs` + pair `Pool {}`) — never indexer USD, never CG `liquidity_in_usd`.
//! Gem pair exclude list mirrors `#562` `COLUMBUS5_GEM_ADDRESSES`.

use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};

/// Soft-launch columbus-5 gem CW20s (same set as `COLUMBUS5_GEM_ADDRESSES` / #562).
/// Volume and pair-linked fees exclude pairs whose either leg matches.
pub const COLUMBUS5_GEM_ADDRESSES: &[&str] = &[
    "terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94", // EMBER
    "terra1k6cqupylk0wp4pj273pntwhv9py0q5guyqye8ssvukn9xq7mes7sdlmena", // CORAL
    "terra1ejq3mjjgnklpa3pg4jterlfwsny055gpmcjf3fz0ev3ueajnzeysz6xxgr", // JADE
    "terra178fgrfzv7njtmdp9vghyf2dx77sah8u8jluzs7ym562chaxnmj2s6mn6m9", // ONYX
    "terra1fga508hzx8dd7x8q4uhm6mdhkqv6fxrtsea3r27smdqmv5k2jgxq5zk9fc", // RUBY
    "terra12k67cvfs7y7g8lca3qr4g4py6s6j69fu24gze5pjfamfpckv8mps7cymme", // TOPAZ
    "terra17dpnjlpgsnm8muu4msfjra4f2hrptnjp2jdpkka4p0e3px42ayxq0pmc2z", // QUARTZ
    "terra18fzufz8cs7ez49xjwgs248x85za5v50yug55fj7lyxp9hapxyr7qnh3czs", // PEARL
];

/// Refresh today plus this many prior UTC days (late ingest / TZ edges).
pub const DAILY_LOOKBACK_DAYS: i64 = 8;

/// Columbus-5 factory pin (REGISTRY.md). TVL adapter only — not an indexer USD source.
pub const COLUMBUS5_FACTORY: &str =
    "terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea";

/// First UTC day the dimension adapters may request (soft-launch window).
pub const ADAPTER_START_UTC_DAY: i64 = 1_777_593_600; // 2026-05-01 00:00:00 UTC

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseDailyError {
    Invalid,
    Unaligned,
    Future,
}

impl ParseDailyError {
    pub fn as_http_message(self) -> &'static str {
        match self {
            ParseDailyError::Invalid => "Invalid timestamp: expected unix seconds of 00:00 UTC",
            ParseDailyError::Unaligned => "timestamp must be 00:00 UTC (unix % 86400 == 0)",
            ParseDailyError::Future => "timestamp must not be a future UTC day",
        }
    }
}

/// Parse a single UTC calendar-day unix timestamp. Rejects non-i64, negative,
/// unaligned, and days after today UTC. No SQL interpolation — caller binds i64.
pub fn parse_utc_day_timestamp(raw: &str) -> Result<i64, ParseDailyError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ParseDailyError::Invalid);
    }
    if trimmed.chars().any(|c| !c.is_ascii_digit() && c != '-') {
        return Err(ParseDailyError::Invalid);
    }
    let ts: i64 = trimmed.parse().map_err(|_| ParseDailyError::Invalid)?;
    if ts < 0 {
        return Err(ParseDailyError::Invalid);
    }
    if ts % 86_400 != 0 {
        return Err(ParseDailyError::Unaligned);
    }
    let today_start = utc_day_start(Utc::now());
    if ts > today_start.timestamp() {
        return Err(ParseDailyError::Future);
    }
    Ok(ts)
}

pub fn utc_day_start(now: DateTime<Utc>) -> DateTime<Utc> {
    now.date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|naive| Utc.from_utc_datetime(&naive))
        .expect("00:00:00 is a valid naive time")
}

pub fn naive_utc_day(ts: i64) -> Option<NaiveDate> {
    DateTime::from_timestamp(ts, 0).map(|dt| dt.date_naive())
}

/// Same null / `"0"` contract as overview volume (#548 / #631).
/// Activity + unpriced → `None` (JSON null). Idle → `"0"`. Never silent zero with trades.
pub fn daily_usd_field(activity_count: i64, priced_usd: &BigDecimal) -> Option<String> {
    if activity_count <= 0 {
        return Some("0".to_string());
    }
    if priced_usd <= &BigDecimal::from(0) {
        return None;
    }
    Some(priced_usd.to_string())
}

/// Fail-closed aggregate: any unpriced activity in the dimension → null.
pub fn daily_usd_field_fail_closed(
    activity_count: i64,
    unpriced_count: i64,
    priced_usd: &BigDecimal,
) -> Option<String> {
    if activity_count <= 0 {
        return Some("0".to_string());
    }
    if unpriced_count > 0 || priced_usd <= &BigDecimal::from(0) {
        return None;
    }
    Some(priced_usd.to_string())
}

pub fn gem_addresses_lowercased() -> Vec<String> {
    COLUMBUS5_GEM_ADDRESSES
        .iter()
        .map(|a| a.to_ascii_lowercase())
        .collect()
}

pub fn is_columbus5_gem_address(addr: &str) -> bool {
    let lower = addr.trim().to_ascii_lowercase();
    COLUMBUS5_GEM_ADDRESSES
        .iter()
        .any(|g| g.eq_ignore_ascii_case(&lower))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn parse_rejects_injection_and_unaligned() {
        assert_eq!(
            parse_utc_day_timestamp("1;drop table"),
            Err(ParseDailyError::Invalid)
        );
        assert_eq!(
            parse_utc_day_timestamp("window=1;drop table"),
            Err(ParseDailyError::Invalid)
        );
        assert_eq!(parse_utc_day_timestamp("-1"), Err(ParseDailyError::Invalid));
        assert_eq!(
            parse_utc_day_timestamp("86401"),
            Err(ParseDailyError::Unaligned)
        );
        assert_eq!(parse_utc_day_timestamp(""), Err(ParseDailyError::Invalid));
        assert_eq!(
            parse_utc_day_timestamp("1746057600.5"),
            Err(ParseDailyError::Invalid)
        );
    }

    #[test]
    fn parse_accepts_aligned_past_and_today() {
        let today = utc_day_start(Utc::now()).timestamp();
        assert_eq!(parse_utc_day_timestamp(&today.to_string()), Ok(today));
        assert_eq!(
            parse_utc_day_timestamp(&ADAPTER_START_UTC_DAY.to_string()),
            Ok(ADAPTER_START_UTC_DAY)
        );
        let tomorrow = today + 86_400;
        assert_eq!(
            parse_utc_day_timestamp(&tomorrow.to_string()),
            Err(ParseDailyError::Future)
        );
        let far = today + 86_400 * 400;
        assert_eq!(
            parse_utc_day_timestamp(&far.to_string()),
            Err(ParseDailyError::Future)
        );
    }

    #[test]
    fn usd_field_null_when_active_unpriced() {
        assert_eq!(
            daily_usd_field(3, &BigDecimal::from(0)),
            None,
            "activity + unpriced must not be \"0\""
        );
        assert_eq!(
            daily_usd_field(0, &BigDecimal::from(0)),
            Some("0".to_string())
        );
        assert_eq!(
            daily_usd_field(2, &BigDecimal::from_str("12.5").unwrap()),
            Some("12.5".to_string())
        );
        assert_eq!(
            daily_usd_field_fail_closed(2, 1, &BigDecimal::from(9)),
            None
        );
    }

    #[test]
    fn gems_match_frontend_562_set() {
        assert_eq!(COLUMBUS5_GEM_ADDRESSES.len(), 8);
        assert!(is_columbus5_gem_address(
            "TERRA1DMURUHHT32X8F47NVM73PWP6Q7UF2JTFHDT3NXCQL4MMQKYFSRAQN2DT94"
        ));
        assert!(!is_columbus5_gem_address(
            "terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72"
        ));
    }
}
