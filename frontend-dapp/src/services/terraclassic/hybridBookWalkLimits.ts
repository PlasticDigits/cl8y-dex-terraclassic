/**
 * On-chain book-walk caps mirrored from `dex-common::pair`.
 * Keep in sync with [`smartcontracts/packages/dex-common/src/pair.rs`](../../../../smartcontracts/packages/dex-common/src/pair.rs).
 *
 * @see [GitLab #254](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/254) scan budget
 * @see [GitLab #260](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/260) frontend gas
 * @see [GitLab #262](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/262) raised caps
 * @see [GitLab #309](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/309) park cap benchmark (retained 15)
 */
export const MAX_MAKER_FILLS_HARD_CAP = 100
/** Hard ceiling on doubly-linked list iterations per book side per hybrid swap (decoupled from maker cap). */
export const MAX_SCAN_STEPS = 500
/** Write-heavy expired-order parks per book walk during hybrid swap (~29k gas/park on optimized wasm; see docs/limit-orders.md § #309). */
export const MAX_EXPIRED_PARKS_PER_SWAP = 15
