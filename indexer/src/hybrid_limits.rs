//! On-chain-aligned hybrid book-walk limits for the indexer (GitLab #379 / #262).
//!
//! Source of truth on-chain: `smartcontracts/packages/dex-common/src/pair.rs`
//! (`MAX_MAKER_FILLS_HARD_CAP`).

/// Max distinct maker fills per hybrid hop — matches on-chain `MAX_MAKER_FILLS_HARD_CAP`.
pub const MAX_MAKER_FILLS_HARD_CAP: u32 = 100;

/// Clamp GET/POST `max_maker_fills` to `[1, MAX_MAKER_FILLS_HARD_CAP]`.
#[inline]
pub fn clamp_max_maker_fills(max_maker_fills: u32) -> u32 {
    max_maker_fills.max(1).min(MAX_MAKER_FILLS_HARD_CAP)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_rejects_zero_and_overflow() {
        assert_eq!(clamp_max_maker_fills(0), 1);
        assert_eq!(clamp_max_maker_fills(8), 8);
        assert_eq!(clamp_max_maker_fills(100), 100);
        assert_eq!(clamp_max_maker_fills(4294967295), MAX_MAKER_FILLS_HARD_CAP);
    }
}
