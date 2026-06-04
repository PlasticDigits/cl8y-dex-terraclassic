//! Unified `max_spread` / `belief_price` slippage checks for pool-only and hybrid swaps.
//!
//! Product rule (GitLab #197): without `belief_price`, compare pool-leg spread metric
//! against **total gross output** (pool gross + book net to taker). With `belief_price`,
//! compare shortfall vs expected output at the belief price using **book net + pool net +
//! pool commission** as actual output. Pool-only swaps are the special case `book_net = 0`.
//!
//! GitLab #273: in the no-belief path the book leg is also bounded — its shortfall against
//! the pool's realized **net** rate (`pool_net_return / pool_input`) is folded into the spread
//! numerator, so a book leg that fills materially worse than the pool can no longer pass by
//! only enlarging the denominator. A **pure-book** hybrid (`pool_input == 0`) has no pool
//! reference rate, so this metric cannot bound it; that case is deferred to the separate
//! `belief_price` / `min_receive` guard (out of scope here).

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
    /// Offer actually routed to the AMM pool (incl. the book's unfilled remainder).
    /// Used as the reference-rate base for the no-belief book-degradation term (#273).
    pub pool_input: Uint128,
    /// Offer actually consumed by book fills (`pool_input + book_input == offer_amount`).
    pub book_input: Uint128,
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
            pool_input: offer_amount,
            book_input: Uint128::zero(),
        }
    }

    /// Hybrid swap with explicit book net to the taker.
    #[allow(clippy::too_many_arguments)]
    pub fn hybrid(
        offer_amount: Uint128,
        pool_net_return: Uint128,
        pool_commission: Uint128,
        pool_spread: Uint128,
        book_net_return: Uint128,
        pool_input: Uint128,
        book_input: Uint128,
    ) -> Self {
        Self {
            offer_amount,
            pool_net_return,
            pool_commission,
            pool_spread,
            book_net_return,
            pool_input,
            book_input,
        }
    }

    /// Build from a hybrid simulation / execute snapshot.
    #[allow(clippy::too_many_arguments)]
    pub fn from_hybrid_simulation(
        offer_amount: Uint128,
        pool_return_amount: Uint128,
        pool_commission_amount: Uint128,
        spread_amount: Uint128,
        book_return_amount: Uint128,
        pool_input: Uint128,
        book_input: Uint128,
    ) -> Self {
        Self::hybrid(
            offer_amount,
            pool_return_amount,
            pool_commission_amount,
            spread_amount,
            book_return_amount,
            pool_input,
            book_input,
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

        // GitLab #273: bound the book leg too. Its shortfall against the pool's realized net
        // rate (pool_net_return / pool_input) is folded into the numerator, so a book leg
        // filling materially worse than the pool can no longer pass by only enlarging the
        // denominator. Gated to exactly zero unless both legs are present, preserving the
        // #197 pool-only metric byte-for-byte. pool_input == 0 (pure-book) has no pool
        // reference and is left to the separate belief/min_receive guard.
        let book_shortfall = if inputs.pool_input.is_zero() || inputs.book_input.is_zero() {
            Uint128::zero()
        } else {
            let fair_net_book = inputs
                .pool_net_return
                .checked_multiply_ratio(inputs.book_input, inputs.pool_input)
                .map_err(|_| MaxSpreadViolation {
                    max_allowed,
                    actual: Decimal::zero(),
                })?;
            fair_net_book.saturating_sub(inputs.book_net_return)
        };
        let spread_total = spread_cmp
            .checked_add(book_shortfall)
            .map_err(|_| MaxSpreadViolation {
                max_allowed,
                actual: Decimal::zero(),
            })?;

        if total_gross_out > Uint128::zero()
            && Decimal::from_ratio(spread_total, total_gross_out) > max_allowed
        {
            return Err(MaxSpreadViolation {
                max_allowed,
                actual: Decimal::from_ratio(spread_total, total_gross_out),
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
            Uint128::new(1),
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
                Uint128::new(1),
                Uint128::zero(),
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
                Uint128::new(1),
                Uint128::zero(),
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
            &MaxSpreadInputs::hybrid(
                offer,
                pool_net,
                pool_commission,
                Uint128::zero(),
                book_net,
                offer,
                Uint128::zero(),
            ),
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
                offer,
                Uint128::zero(),
            ),
        )
        .is_err());
    }

    // GitLab #273 — no-belief branch must reflect book-leg degradation vs the pool net rate.
    #[test]
    fn no_belief_rejects_book_far_below_pool_net_rate() {
        // Pool leg 1000 in -> 997 net (+3 commission, 1 spread); book leg 10000 in -> 4985
        // net (filled ~50% below the pool rate). fair_net_book = 997*10000/1000 = 9970,
        // shortfall = 9970-4985 = 4985, spread_total ~4986 / total_gross ~5985 ~= 0.83 -> reject.
        let inputs = MaxSpreadInputs::hybrid(
            Uint128::new(11_000),
            Uint128::new(997),
            Uint128::new(3),
            Uint128::new(1),
            Uint128::new(4985),
            Uint128::new(1_000),  // pool_input
            Uint128::new(10_000), // book_input
        );
        let err = check_max_spread(None, Some(Decimal::percent(1)), &inputs).unwrap_err();
        assert!(err.actual > err.max_allowed);
    }

    // GitLab #273 — a legit hybrid whose book fills at ~the pool net rate (book only short by
    // its own fee) must still pass: net-vs-net means the book fee never registers as shortfall.
    #[test]
    fn no_belief_accepts_book_at_pool_net_rate() {
        // Pool 6000 in -> 5947 net (+17 commission, 36 spread); book 4000 in -> 3996 net
        // (book at pool spot minus ~10bps fee). fair_net_book = 5947*4000/6000 = 3964 <= 3996
        // -> shortfall 0; metric = pool spread only, well under 1%.
        let inputs = MaxSpreadInputs::hybrid(
            Uint128::new(10_000),
            Uint128::new(5947),
            Uint128::new(17),
            Uint128::new(36),
            Uint128::new(3996),
            Uint128::new(6_000),
            Uint128::new(4_000),
        );
        check_max_spread(None, Some(Decimal::percent(1)), &inputs).unwrap();
    }
}
