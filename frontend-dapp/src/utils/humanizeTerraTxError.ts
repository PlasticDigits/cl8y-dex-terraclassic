/**
 * Map noisy chain / LCD errors to short retail copy before surfacing in the UI.
 * Raw logs stay in `console.error` upstream ([GitLab #134](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134), [GitLab #135](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/135)).
 * Wallet / fetch / indexer transport copy is classified in `humanizeOffChainError.ts` and composed via `humanizeUserFacingError.ts` ([GitLab #145](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/145)).
 */

import { humanizeCosmwasmLimitOrderMissingMessage } from './limitOrderCancelUserMessage'
import { humanizeExpiredLimitClaimMessage } from './limitClaimUserMessage'

/** Strip repeated `Transaction failed:` prefixes from nested throws. */
export function stripNestedTransactionFailedPrefixes(message: string): string {
  let s = message.trim()
  const prefix = /^Transaction failed:\s*/i
  while (prefix.test(s)) {
    s = s.replace(prefix, '').trim()
  }
  return s
}

/**
 * If the message matches a known pattern, return a human-readable replacement.
 * Otherwise return `null` so callers keep the (possibly stripped) original.
 */
export function tryHumanizeTerraTxMessage(message: string): string | null {
  const inner = stripNestedTransactionFailedPrefixes(message)
  const limitOrder = humanizeCosmwasmLimitOrderMissingMessage(inner)
  if (limitOrder) {
    return limitOrder
  }
  const claim = humanizeExpiredLimitClaimMessage(inner)
  if (claim) {
    return claim
  }
  if (/Max spread assertion/i.test(inner)) {
    return (
      'Trade rejected: price impact exceeds your slippage tolerance on at least one pool hop. ' +
      'Try a smaller amount, pick a deeper pool route, or raise slippage tolerance in Settings (higher slippage increases execution risk).'
    )
  }
  if (/assert_not_paused|contract is paused/i.test(inner)) {
    return (
      'This pool is currently paused by the operator. Try again later or pick a different pair.'
    )
  }
  if (/\bUnauthorized\b/i.test(inner)) {
    return (
      'You do not have permission for this action.'
    )
  }
  if (/insufficient funds/i.test(inner)) {
    return (
      'Insufficient LUNC for transaction fees. Top up your wallet and try again.'
    )
  }
  if (/out of gas/i.test(inner)) {
    return (
      'Transaction needed more gas than estimated. Try again — gas usage can vary slightly between blocks.'
    )
  }
  if (/assert_deadline|deadline exceeded/i.test(inner)) {
    return (
      'Transaction took too long to confirm and the deadline was reached. Try again.'
    )
  }
  if (/InvariantViolation/i.test(inner)) {
    return (
      'Pool state inconsistency detected. Refresh the page and try again. If this keeps happening, the pool may need operator attention.'
    )
  }
  return null
}
