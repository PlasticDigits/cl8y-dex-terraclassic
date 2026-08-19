/**
 * Wallet broadcast / LCD poll timeouts for Terra Classic txs ([GitLab #173](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/173)).
 * Post-sign broadcast recovery ([GitLab #359](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/359)).
 * Keplr / Ledger sign-stall wait ([GitLab #567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567)) — not the 30s broadcast cap.
 */

const DEFAULT_BROADCAST_MS = 30_000
const DEFAULT_POLL_MS = 90_000
/** Ledger / Keplr extension sign wait — minutes, not the 30s RPC cap (GitLab #567). */
const DEFAULT_SIGN_MS = 240_000
/** Delayed Keplr signing hint for software extension (GitLab #567). */
const DEFAULT_SIGNING_HINT_DELAY_MS = 12_000
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

/**
 * Max wait for split-path `signTerraTxRaw` on Keplr extension / Ledger (GitLab #567).
 * Must not reuse {@link TERRA_TX_BROADCAST_TIMEOUT_MS} — Ledger confirmations take minutes (K567-5).
 */
export const TERRA_TX_SIGN_TIMEOUT_MS = parsePositiveIntEnv(
  import.meta.env.VITE_TERRA_TX_SIGN_TIMEOUT_MS,
  DEFAULT_SIGN_MS
)

/** After this elapsed signing time, software Keplr sees a generic approve/refresh hint (K567-4). */
export const TERRA_TX_SIGNING_HINT_DELAY_MS = parsePositiveIntEnv(
  import.meta.env.VITE_TERRA_TX_SIGNING_HINT_DELAY_MS,
  DEFAULT_SIGNING_HINT_DELAY_MS
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

/** Immediate signing-phase hint for detected Ledger (GitLab #567). No coin types, seed, or PIN. */
export const TERRA_TX_SIGNING_LEDGER_HINT =
  'Open the Terra Classic (LUNA) app on your Ledger — not Cosmos — then approve in Keplr. If it stays blank, refresh Terra Classic in Keplr.'

/** Delayed software-Keplr hint after {@link TERRA_TX_SIGNING_HINT_DELAY_MS} (GitLab #567). */
export const TERRA_TX_SIGNING_KEPLR_DELAYED_HINT =
  'Approve in Keplr if a popup is open. If it stays blank, refresh Terra Classic in Keplr. Using a Ledger? Open the Terra Classic (LUNA) app — not Cosmos.'

/** Pre-sign hang on detected Ledger — safe to retry (no signed bytes). Not #173 copy (K567-5). */
export const TERRA_TX_SIGN_STALL_LEDGER_MESSAGE =
  'Signing is taking too long. Open the Terra Classic (LUNA) app on your Ledger — not Cosmos — unlock the device, then refresh Terra Classic in Keplr and try again.'

/** Pre-sign hang on software Keplr extension — safe to retry (no signed bytes). Not #173 copy (K567-5). */
export const TERRA_TX_SIGN_STALL_KEPLR_MESSAGE =
  'Signing is taking too long. Approve in Keplr if a popup is open. If it stays blank, refresh Terra Classic in Keplr and try again.'

export function terraTxSignStallMessage(isNanoLedger: boolean): string {
  return isNanoLedger ? TERRA_TX_SIGN_STALL_LEDGER_MESSAGE : TERRA_TX_SIGN_STALL_KEPLR_MESSAGE
}

export function isTerraTxSignStallMessage(message: string): boolean {
  return message === TERRA_TX_SIGN_STALL_LEDGER_MESSAGE || message === TERRA_TX_SIGN_STALL_KEPLR_MESSAGE
}

export function isTerraTxTimeoutMessage(message: string): boolean {
  return (
    message === TERRA_TX_BROADCAST_TIMEOUT_MESSAGE ||
    message === TERRA_TX_POLL_TIMEOUT_MESSAGE ||
    message === TERRA_TX_POST_SIGN_BROADCAST_UNKNOWN_MESSAGE ||
    message === TERRA_TX_POST_SIGN_NOT_FOUND_MESSAGE ||
    isTerraTxSignStallMessage(message)
  )
}
