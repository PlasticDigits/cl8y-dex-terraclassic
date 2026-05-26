//! Unified `max_spread` / `belief_price` slippage checks for pool-only and hybrid swaps.
//!
//! Product rule (GitLab #197): without `belief_price`, compare pool-leg spread metric
//! against **total gross output** (pool gross + book net to taker). With `belief_price`,
//! compare shortfall vs expected output at the belief price using **book net + pool net +
//! pool commission** as actual output. Pool-only swaps are the special case `book_net = 0`.

use cosmwasm_std::{Decimal, Uint128};

/// Default slippage tolerance when the user omits `max_spread` (1%).
pub fn default_max_spread() -> Decimal {
    Decimal::percent(1)
}

/// Leg amounts passed into the spread check after book + pool settlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxSpreadInputs {
    pub offer_amount: Uint128,
    pub pool_net_return: Uint128,
    pub pool_commission: Uint128,
    pub pool_spread: Uint128,
    pub book_net_return: Uint128,
}

impl MaxSpreadInputs {
    /// Pool-only swap (`book_net_return = 0`).
    pub fn pool_only(
        offer_amount: Uint128,
        pool_net_return: Uint128,
        pool_commission: Uint128,
        pool_spread: Uint128,
    ) -> Self {
        Self {
            offer_amount,
            pool_net_return,
            pool_commission,
            pool_spread,
            book_net_return: Uint128::zero(),
        }
    }

    /// Hybrid swap with explicit book net to the taker.
    pub fn hybrid(
        offer_amount: Uint128,
        pool_net_return: Uint128,
        pool_commission: Uint128,
        pool_spread: Uint128,
        book_net_return: Uint128,
    ) -> Self {
        Self {
            offer_amount,
            pool_net_return,
            pool_commission,
            pool_spread,
            book_net_return,
        }
    }

    /// Build from a hybrid simulation / execute snapshot.
    pub fn from_hybrid_simulation(
        offer_amount: Uint128,
        pool_return_amount: Uint128,
        commission_amount: Uint128,
        spread_amount: Uint128,
        book_return_amount: Uint128,
    ) -> Self {
        Self::hybrid(
            offer_amount,
            pool_return_amount,
            commission_amount,
            spread_amount,
            book_return_amount,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaxSpreadViolation {
    pub max_allowed: Decimal,
    pub actual: Decimal,
}

/// Returns `Ok(())` when spread is within tolerance; `Err` when it strictly exceeds `max_spread`.
pub fn check_max_spread(
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    inputs: &MaxSpreadInputs,
) -> Result<(), MaxSpreadViolation> {
    let max_allowed = max_spread.unwrap_or_else(default_max_spread);

    if let Some(bp) = belief_price {
        let expected_return = inputs.offer_amount * (Decimal::one() / bp);
        let actual_return = inputs
            .book_net_return
            .checked_add(inputs.pool_net_return)
            .and_then(|v| v.checked_add(inputs.pool_commission))
            .map_err(|_| MaxSpreadViolation {
                max_allowed,
                actual: Decimal::zero(),
            })?;
        let spread = if expected_return > actual_return {
            expected_return - actual_return
        } else {
            Uint128::zero()
        };

        if expected_return > Uint128::zero()
            && Decimal::from_ratio(spread, expected_return) > max_allowed
        {
            return Err(MaxSpreadViolation {
                max_allowed,
                actual: Decimal::from_ratio(spread, expected_return),
            });
        }
    } else {
        let pool_gross = inputs
            .pool_net_return
            .checked_add(inputs.pool_commission)
            .map_err(|_| MaxSpreadViolation {
                max_allowed,
                actual: Decimal::zero(),
            })?;
        let total_gross_out = pool_gross
            .checked_add(inputs.book_net_return)
            .map_err(|_| MaxSpreadViolation {
                max_allowed,
                actual: Decimal::zero(),
            })?;
        let spread_cmp = inputs.pool_spread.min(pool_gross);
        if total_gross_out > Uint128::zero()
            && Decimal::from_ratio(spread_cmp, total_gross_out) > max_allowed
        {
            return Err(MaxSpreadViolation {
                max_allowed,
                actual: Decimal::from_ratio(spread_cmp, total_gross_out),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_only_matches_hybrid_with_zero_book() {
        let inputs = MaxSpreadInputs::pool_only(
            Uint128::new(1),
            Uint128::new(90),
            Uint128::new(10),
            Uint128::new(5),
        );
        let hybrid_inputs = MaxSpreadInputs::hybrid(
            Uint128::new(1),
            Uint128::new(90),
            Uint128::new(10),
            Uint128::new(5),
            Uint128::zero(),
        );
        let max = Some(Decimal::percent(5));
        assert_eq!(
            check_max_spread(None, max, &inputs),
            check_max_spread(None, max, &hybrid_inputs)
        );
    }

    #[test]
    fn max_spread_one_allows_capped_ratio_when_raw_spread_exceeds_gross() {
        check_max_spread(
            None,
            Some(Decimal::one()),
            &MaxSpreadInputs::pool_only(
                Uint128::new(1),
                Uint128::new(90),
                Uint128::new(10),
                Uint128::new(150),
            ),
        )
        .unwrap();
    }

    #[test]
    fn max_spread_half_rejects_at_full_slippage_metric() {
        let err = check_max_spread(
            None,
            Some(Decimal::percent(50)),
            &MaxSpreadInputs::pool_only(
                Uint128::new(1),
                Uint128::new(90),
                Uint128::new(10),
                Uint128::new(150),
            ),
        )
        .unwrap_err();
        assert!(err.actual > err.max_allowed);
    }

    #[test]
    fn no_belief_denominator_includes_book_net() {
        let book = Uint128::new(50);
        let pool_gross = Uint128::new(100);
        let spread = Uint128::new(5);
        let denom = pool_gross.checked_add(book).unwrap();
        let exact_max = Decimal::from_ratio(spread.min(pool_gross), denom);
        check_max_spread(
            None,
            Some(exact_max),
            &MaxSpreadInputs::hybrid(
                Uint128::new(1),
                Uint128::new(90),
                Uint128::new(10),
                spread,
                book,
            ),
        )
        .unwrap();
        let tighter = Decimal::from_ratio(4u128, 150u128);
        assert!(check_max_spread(
            None,
            Some(tighter),
            &MaxSpreadInputs::hybrid(
                Uint128::new(1),
                Uint128::new(90),
                Uint128::new(10),
                spread,
                book,
            ),
        )
        .is_err());
    }

    #[test]
    fn belief_counts_pool_commission_in_actual_return() {
        let offer = Uint128::new(100);
        let belief_price = Decimal::from_ratio(Uint128::new(1), Uint128::new(2));
        let book_net = Uint128::new(10);
        let pool_net = Uint128::new(170);
        let pool_commission = Uint128::new(20);
        let max_tight = Decimal::permille(4);
        check_max_spread(
            Some(belief_price),
            Some(max_tight),
            &MaxSpreadInputs::hybrid(offer, pool_net, pool_commission, Uint128::zero(), book_net),
        )
        .unwrap();
        assert!(check_max_spread(
            Some(belief_price),
            Some(max_tight),
            &MaxSpreadInputs::hybrid(
                offer,
                pool_net.checked_sub(Uint128::new(1)).unwrap(),
                pool_commission,
                Uint128::zero(),
                book_net,
            ),
        )
        .is_err());
    }
}
