//! Canonical fee-discount tiers aligned with `docs/reference/fee-discount-tiers.md`.

pub type TierRow = (u8, u128, u16, bool);

/// Production-style tier ladder (tiers 0–9 and 255). `u128` is `min_cl8y_balance` in base units.
pub const STANDARD_PRODUCTION_TIERS: &[TierRow] = &[
    (0, 0, 10_000, true),
    (1, 1_000_000_000_000_000_000, 250, false),
    (2, 5_000_000_000_000_000_000, 1_000, false),
    (3, 20_000_000_000_000_000_000, 2_000, false),
    (4, 75_000_000_000_000_000_000, 3_500, false),
    (5, 200_000_000_000_000_000_000, 5_000, false),
    (6, 500_000_000_000_000_000_000, 6_000, false),
    (7, 1_500_000_000_000_000_000_000, 7_500, false),
    (8, 3_500_000_000_000_000_000_000, 8_500, false),
    (9, 7_500_000_000_000_000_000_000, 9_500, false),
    (255, 0, 0, true),
];
