/**
 * Cosmos SDK account-sequence helpers for CheckTx code-32 recovery ([GitLab #499](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/499)).
 *
 * Sequence mismatch is a definite mempool rejection (tx never entered) — safe to re-sign once
 * with the chain-reported expected sequence. Do not confuse with post-sign #359 recovery.
 */

import type { ConnectedWallet } from '@goblinhunt/cosmes/wallet'

const SEQUENCE_MISMATCH_RE = /account sequence mismatch|incorrect account sequence/i

/** Cosmes-style strict parse: `expected N, got M:` → N. */
const EXPECTED_SEQUENCE_RE = /account sequence mismatch,\s*expected\s+(\d+),\s*got\s+(\d+)\s*:/i

export function isAccountSequenceMismatchError(error: unknown): boolean {
  if (!(error instanceof Error)) return false
  return SEQUENCE_MISMATCH_RE.test(error.message)
}

/**
 * Returns the chain-expected sequence from a code-32 error, or `null` if the message
 * is not a parseable account sequence mismatch.
 */
export function extractExpectedAccountSequence(error: unknown): bigint | null {
  if (!(error instanceof Error)) return null
  const matches = error.message.match(EXPECTED_SEQUENCE_RE)
  if (!matches?.[1]) return null
  return BigInt(matches[1])
}

/** Overwrite cosmes wallet sequence cache (used after parsing code-32 expected value). */
export function setWalletCachedSequence(wallet: ConnectedWallet, sequence: bigint): void {
  const w = wallet as unknown as { sequence?: bigint }
  w.sequence = sequence
}

/** Drop cached sequence so the next `getAuthInfo(false)` must refresh from chain. */
export function clearWalletCachedSequence(wallet: ConnectedWallet): void {
  const w = wallet as unknown as { sequence?: bigint }
  w.sequence = undefined
}

export type AccountSequenceRetryPlan = {
  /** Pass to `signTerraTxRaw` — true when expected sequence was applied from the error. */
  useCachedSequence: boolean
}

/**
 * If `error` is a sequence mismatch, prepare the wallet for one re-sign attempt.
 * Prefers the chain-reported expected sequence (LCD may still be stale).
 */
export function planAccountSequenceRetry(wallet: ConnectedWallet, error: unknown): AccountSequenceRetryPlan | null {
  if (!isAccountSequenceMismatchError(error)) return null

  const expected = extractExpectedAccountSequence(error)
  if (expected !== null) {
    setWalletCachedSequence(wallet, expected)
    return { useCachedSequence: true }
  }

  clearWalletCachedSequence(wallet)
  return { useCachedSequence: false }
}
