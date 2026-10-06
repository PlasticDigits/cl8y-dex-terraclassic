//! Bounded exact financial values from untrusted LCD attributes (issue #1395).
//!
//! Raw token amounts fit `NUMERIC(38, 0)`: ASCII unsigned integers of at most 38
//! bytes. Zero is a real value. Missing optional values remain `None`; conversion
//! failure is never replaced with zero. Prices fit `NUMERIC(38, 18)` exactly.
use bigdecimal::BigDecimal;
use std::fmt;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountError {
    Missing,
    Invalid,
    Overflow,
}

impl fmt::Display for AmountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Missing => "missing",
            Self::Invalid => "invalid",
            Self::Overflow => "overflow",
        })
    }
}
impl std::error::Error for AmountError {}

pub fn parse_raw_amount(value: Option<&str>) -> Result<BigDecimal, AmountError> {
    let value = value.ok_or(AmountError::Missing)?;
    // Bound work before inspecting or allocating the supplied payload.
    if value.len() > 38 {
        return Err(AmountError::Overflow);
    }
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AmountError::Invalid);
    }
    value.parse().map_err(|_| AmountError::Invalid)
}

pub fn parse_optional_raw_amount(value: Option<&str>) -> Result<Option<BigDecimal>, AmountError> {
    value.map(|v| parse_raw_amount(Some(v))).transpose()
}

pub fn parse_price(value: Option<&str>) -> Result<BigDecimal, AmountError> {
    let value = value.ok_or(AmountError::Missing)?;
    if value.len() > 39 {
        return Err(AmountError::Overflow);
    }
    let (integer, fraction) = match value.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (value, None),
    };
    if integer.is_empty()
        || !integer.bytes().all(|b| b.is_ascii_digit())
        || fraction.is_some_and(|f| f.is_empty() || !f.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(AmountError::Invalid);
    }
    if integer.len() > 20 || fraction.is_some_and(|f| f.len() > 18) {
        return Err(AmountError::Overflow);
    }
    value.parse().map_err(|_| AmountError::Invalid)
}

/// Report only fixed event/field names and a typed reason; never raw values or identities.
pub fn report_amount_rejection(kind: &'static str, field: &'static str, error: AmountError) {
    static BUDGET: OnceLock<Mutex<RejectionLogBudget>> = OnceLock::new();
    let mut budget = BUDGET
        .get_or_init(|| Mutex::new(RejectionLogBudget::new(Instant::now())))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(suppressed) = budget.take(Instant::now()) else {
        return;
    };
    drop(budget);
    tracing::warn!(event_kind = kind, field, reason = %error, suppressed,
        "Rejected malformed economic event; diagnostics limited to 8 per minute");
}

/// Process-wide bound, shared by every parser and unaffected by tx/block replay.
struct RejectionLogBudget {
    window_start: Instant,
    emitted: u8,
    suppressed: u64,
}

impl RejectionLogBudget {
    fn new(now: Instant) -> Self {
        Self {
            window_start: now,
            emitted: 0,
            suppressed: 0,
        }
    }

    fn take(&mut self, now: Instant) -> Option<u64> {
        let mut prior_suppressed = 0;
        if now.duration_since(self.window_start) >= Duration::from_secs(60) {
            self.window_start = now;
            self.emitted = 0;
            prior_suppressed = std::mem::take(&mut self.suppressed);
        }
        if self.emitted < 8 {
            self.emitted += 1;
            Some(prior_suppressed)
        } else {
            self.suppressed = self.suppressed.saturating_add(1);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_poison_diagnostics_are_bounded_and_report_suppressed_count() {
        let now = Instant::now();
        let mut budget = RejectionLogBudget::new(now);
        for _ in 0..8 {
            assert_eq!(budget.take(now), Some(0));
        }
        for _ in 0..100_000 {
            assert_eq!(budget.take(now + Duration::from_secs(59)), None);
        }
        assert_eq!(budget.take(now + Duration::from_secs(60)), Some(100_000));
        assert_eq!(budget.take(now + Duration::from_secs(60)), Some(0));
    }

    #[test]
    fn strict_raw_amount_domain_and_typed_errors() {
        assert_eq!(parse_raw_amount(None), Err(AmountError::Missing));
        for value in ["", "-1", "+1", " 1", "1 ", "1.0", "1e2", "１２"] {
            assert_eq!(
                parse_raw_amount(Some(value)),
                Err(AmountError::Invalid),
                "{value}"
            );
        }
        let max = "9".repeat(38);
        assert_eq!(parse_raw_amount(Some(&max)).unwrap().to_string(), max);
        assert_eq!(parse_raw_amount(Some("0")).unwrap(), BigDecimal::from(0));
        for value in [
            "1".repeat(39),
            "0".repeat(1_000_000),
            "1e".to_string() + &"9".repeat(1_000_000),
        ] {
            assert_eq!(parse_raw_amount(Some(&value)), Err(AmountError::Overflow));
        }
        assert_eq!(parse_optional_raw_amount(None).unwrap(), None);
        assert_eq!(
            parse_optional_raw_amount(Some("0")).unwrap(),
            Some(BigDecimal::from(0))
        );
        assert_eq!(
            parse_optional_raw_amount(Some("bad")),
            Err(AmountError::Invalid)
        );
    }

    #[test]
    fn decimal_prices_are_exact_and_database_bounded() {
        let max = "99999999999999999999.999999999999999999";
        assert_eq!(parse_price(Some(max)).unwrap().to_string(), max);
        for value in ["-1", " 1", "1e2", "1.", ".1", "1.2.3"] {
            assert_eq!(parse_price(Some(value)), Err(AmountError::Invalid));
        }
        for value in ["100000000000000000000", "0.0000000000000000001"] {
            assert_eq!(parse_price(Some(value)), Err(AmountError::Overflow));
        }
    }
}
