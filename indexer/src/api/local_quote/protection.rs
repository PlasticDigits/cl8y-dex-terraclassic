//! Quote-relative net floors and finite L9 headroom (#1473).
//! All ratios use checked wide integers; the legacy floor remains available.
use super::{diagnostics, BigInt, Snapshot, SCALE};

pub(super) fn net_floor(s: &Snapshot, output: u128, tolerance: u128) -> Option<u128> {
    if tolerance > SCALE {
        return None;
    }
    let n = BigInt::from(output) * BigInt::from(SCALE - tolerance);
    let d = BigInt::from(SCALE);
    let amount = if s.quote_relative_protection {
        (n + &d - BigInt::from(1u8)) / d
    } else {
        n / d
    };
    amount
        .to_string()
        .parse::<u128>()
        .ok()
        .map(|floor| floor.max(1))
}
/// Existing router L9 ceiling for a pinned quote-relative tolerance u:
/// (r + u) / (1 - u), where r is the original market numerator/denominator.
/// Round upward in atomics so accepted exact boundaries remain accepted.
pub(super) fn quote_relative_market_spread(
    numerator: u128,
    denominator: u128,
    tolerance: u128,
) -> Result<u128, diagnostics::Rejection> {
    if denominator == 0 {
        return Err(diagnostics::Rejection::Dust);
    }
    if numerator > denominator || tolerance >= SCALE {
        return Err(diagnostics::Rejection::Protection);
    }
    let n = (BigInt::from(numerator) * BigInt::from(SCALE)
        + BigInt::from(denominator) * BigInt::from(tolerance))
        * BigInt::from(SCALE);
    let d = BigInt::from(denominator) * BigInt::from(SCALE - tolerance);
    let ceiling = ((n + &d - BigInt::from(1u8)) / d)
        .to_string()
        .parse::<u128>()
        .map_err(|_| diagnostics::Rejection::Accounting)?;
    Ok(ceiling.min(SCALE))
}
pub(super) fn decimal_text(atomics: u128) -> String {
    if atomics == SCALE {
        return "1".into();
    }
    format!("0.{atomics:018}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::super::tests::fixture;
    use super::*;

    #[test]
    fn issue1473_headroom_and_integer_floors_are_bounded_and_exact() {
        assert_eq!(
            quote_relative_market_spread(10, 100, SCALE / 20).unwrap(),
            157_894_736_842_105_264
        );
        assert_eq!(
            quote_relative_market_spread(10, 100, 0).unwrap(),
            SCALE / 10
        );
        assert_eq!(
            quote_relative_market_spread(90, 100, SCALE * 30 / 100).unwrap(),
            SCALE
        );
        assert!(matches!(
            quote_relative_market_spread(0, 0, SCALE / 20),
            Err(diagnostics::Rejection::Dust)
        ));
        assert!(matches!(
            quote_relative_market_spread(101, 100, SCALE / 20),
            Err(diagnostics::Rejection::Protection)
        ));
        assert!(matches!(
            quote_relative_market_spread(0, 100, SCALE),
            Err(diagnostics::Rejection::Protection)
        ));
        assert_eq!(
            quote_relative_market_spread(u128::MAX / 2, u128::MAX, SCALE / 20).unwrap(),
            578_947_368_421_052_632
        );
        let mut s = fixture();
        s.quote_relative_protection = true;
        for net in [1, 2, 19, 20, 21, u128::MAX] {
            let floor = net_floor(&s, net, SCALE / 20).unwrap();
            assert!(
                BigInt::from(floor) * BigInt::from(SCALE)
                    >= BigInt::from(net) * BigInt::from(SCALE - SCALE / 20)
            );
            if floor > 0 {
                assert!(
                    BigInt::from(floor - 1) * BigInt::from(SCALE)
                        < BigInt::from(net) * BigInt::from(SCALE - SCALE / 20)
                );
            }
        }
        assert_eq!(net_floor(&s, 2, SCALE / 20), Some(2));
        assert_eq!(net_floor(&s, 19, SCALE / 20), Some(19));
    }
}
