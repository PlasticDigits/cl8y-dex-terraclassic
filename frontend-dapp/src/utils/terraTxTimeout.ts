/**
 * Wallet broadcast / LCD poll timeouts for Terra Classic txs ([GitLab #173](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/173)).
 * Post-sign broadcast recovery ([GitLab #359](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/359)).
 */

const DEFAULT_BROADCAST_MS = 30_000
const DEFAULT_POLL_MS = 90_000
/** Matches default swap deadline in `useDexStore` when msg has no `deadline`. */
const DEFAULT_RECOVERY_SECONDS = 300

function parsePositiveIntEnv(raw: string | undefined, fallback: number): number {
  if (raw == null || raw === '') return fallback
  const n = Number.parseInt(raw, 10)
  return Number.isFinite(n) && n > 0 ? n : fallback
}

/** Max wait for `wallet.broadcastTx` (wallet RPC / signing transport). */
export const TERRA_TX_BROADCAST_TIMEOUT_MS = parsePositiveIntEnv(
  import.meta.env.VITE_TERRA_TX_BROADCAST_TIMEOUT_MS,
  DEFAULT_BROADCAST_MS
)

/** Max wait for `wallet.pollTx` after a hash is returned. */
export const TERRA_TX_POLL_TIMEOUT_MS = parsePositiveIntEnv(
  import.meta.env.VITE_TERRA_TX_POLL_TIMEOUT_MS,
  DEFAULT_POLL_MS
)

/** Pre-sign / atomic-wallet broadcast transport failure — safe to retry immediately (GitLab #359). */
export const TERRA_TX_BROADCAST_TIMEOUT_MESSAGE =
  'Could not broadcast the transaction. Check your connection and try again.'

export const TERRA_TX_POLL_TIMEOUT_MESSAGE = 'Transaction confirmation timed out. Check your connection and try again.'

export const TERRA_TX_RECOVERY_DEFAULT_SECONDS = DEFAULT_RECOVERY_SECONDS

/** After signing, RPC may still deliver the tx — do not invite retry until recovery poll ends (#359). */
export const TERRA_TX_POST_SIGN_BROADCAST_UNKNOWN_MESSAGE =
  'Broadcast status unknown — the transaction may still confirm. Waiting before you can submit again.'

/** Recovery poll exhausted the msg deadline without finding the tx on chain (#359). */
export const TERRA_TX_POST_SIGN_NOT_FOUND_MESSAGE =
  'The transaction was not found on chain before the deadline. You can try again.'

export function isTerraTxTimeoutMessage(message: string): boolean {
  return (
    message === TERRA_TX_BROADCAST_TIMEOUT_MESSAGE ||
    message === TERRA_TX_POLL_TIMEOUT_MESSAGE ||
    message === TERRA_TX_POST_SIGN_BROADCAST_UNKNOWN_MESSAGE ||
    message === TERRA_TX_POST_SIGN_NOT_FOUND_MESSAGE
  )
}
