//! On-chain parity constants — keep aligned with `smartcontracts/packages/dex-common/src/pair.rs`.

/// Matches on-chain `dex_common::pair::MAX_MAKER_FILLS_HARD_CAP` (GitLab #262, #379).
pub const MAX_MAKER_FILLS_HARD_CAP: u32 = 100;

/// Clamp caller `max_maker_fills` to on-chain bounds (minimum 1, maximum hard cap).
#[inline]
pub fn clamp_max_maker_fills(max_maker_fills: u32) -> u32 {
    max_maker_fills.max(1).min(MAX_MAKER_FILLS_HARD_CAP)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_max_maker_fills_bounds() {
        assert_eq!(clamp_max_maker_fills(0), 1);
        assert_eq!(clamp_max_maker_fills(8), 8);
        assert_eq!(clamp_max_maker_fills(100), 100);
        assert_eq!(clamp_max_maker_fills(4294967295), 100);
    }
}
