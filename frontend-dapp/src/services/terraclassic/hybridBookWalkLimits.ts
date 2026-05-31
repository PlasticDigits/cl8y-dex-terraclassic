/**
 * On-chain book-walk caps mirrored from `dex-common::pair` (read-only; no contract changes in #260).
 * Keep in sync with [`smartcontracts/packages/dex-common/src/pair.rs`](../../../../smartcontracts/packages/dex-common/src/pair.rs).
 *
 * @see [GitLab #254](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/254) scan budget
 * @see [GitLab #260](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/260) frontend gas
 */
export const MAX_MAKER_FILLS_HARD_CAP = 256
export const MAX_SCAN_STEPS_EXTRA = 32
/** Hard ceiling on doubly-linked list iterations per book side per hybrid swap. */
export const MAX_SCAN_STEPS = MAX_MAKER_FILLS_HARD_CAP + MAX_SCAN_STEPS_EXTRA
/** Write-heavy expired-order parks per book walk during hybrid swap. */
export const MAX_EXPIRED_PARKS_PER_SWAP = 15
