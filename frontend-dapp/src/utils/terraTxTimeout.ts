/**
 * Wallet broadcast / LCD poll timeouts for Terra Classic txs ([GitLab #173](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/173)).
 */

const DEFAULT_BROADCAST_MS = 30_000
const DEFAULT_POLL_MS = 90_000

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

export const TERRA_TX_BROADCAST_TIMEOUT_MESSAGE =
  'Could not broadcast the transaction. Check your connection and try again.'

export const TERRA_TX_POLL_TIMEOUT_MESSAGE = 'Transaction confirmation timed out. Check your connection and try again.'
