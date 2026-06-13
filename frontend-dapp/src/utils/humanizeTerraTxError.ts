/**
 * Map noisy chain / LCD errors to short retail copy before surfacing in the UI.
 * Raw logs stay in `console.error` upstream ([GitLab #134](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134), [GitLab #135](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/135)).
 * Wallet / fetch / indexer transport copy is classified in `humanizeOffChainError.ts` and composed via `humanizeUserFacingError.ts` ([GitLab #145](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/145)).
 */

import {
  EXTENSION_SIGNED_FEE_UNDERSHOOT_PREFIX,
  EXTENSION_SIGNED_FEE_UNDERSHOOT_USER_MESSAGE,
} from './extensionSignedFeeGuard'
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
  if (/material pool leg|InsufficientPoolLeg/i.test(inner)) {
    return (
      'Trade rejected: this hybrid route sends too little through the pool while using the limit book without a belief price. ' +
      'Increase the pool leg (at least 10% of the swap), add a belief price, or use pool-only / book-only with appropriate slippage guards.'
    )
  }
  if (/zero net output with book_input|ZeroPoolNetForHybrid/i.test(inner)) {
    return (
      'Trade rejected: the pool leg produced no output while a book leg was requested. ' +
      'Check hybrid split and pool liquidity, or set a belief price for book-heavy routes.'
    )
  }
  if (/assert_not_paused|contract is paused/i.test(inner)) {
    return 'This pool is currently paused by the operator. Try again later or pick a different pair.'
  }
  if (/Trading blacklist/i.test(inner)) {
    return (
      'This action was blocked by the protocol trading blacklist (compliance or incident response). ' +
      'Contact support if you believe this is an error.'
    )
  }
  if (/\bUnauthorized\b/i.test(inner)) {
    return 'You do not have permission for this action.'
  }
  if (
    inner.includes(EXTENSION_SIGNED_FEE_UNDERSHOOT_PREFIX) ||
    inner === EXTENSION_SIGNED_FEE_UNDERSHOOT_USER_MESSAGE
  ) {
    return EXTENSION_SIGNED_FEE_UNDERSHOOT_USER_MESSAGE
  }
  if (/insufficient fees/i.test(inner)) {
    const gotMatch = inner.match(/got:\s*"?(\d+)uluna"?/i)
    const reqMatch = inner.match(/required:[^"]*?(\d+)uluna/i)
    const gotUluna = gotMatch ? BigInt(gotMatch[1]) : 0n
    const reqUluna = reqMatch?.[1] ? BigInt(reqMatch[1]) : 0n
    const staleStationUndershoot = gotUluna > 0n && reqUluna > 0n && gotUluna * 100n < reqUluna * 50n
    if (staleStationUndershoot || /got:\s*"?3000uluna"?/i.test(inner)) {
      return (
        'Terra Station signed a fee far below what this dApp submitted on LocalTerra (built-in ~0.015 uluna/gas; node requires 28.325). ' +
        'Station cannot verify fees on LocalTerra — use Terra Classic Keplr or the dev/simulated wallet. ' +
        'Station P0 belongs on columbus-5 (GitLab #235).'
      )
    }
    return 'Insufficient LUNC for transaction fees. Top up your wallet and try again.'
  }
  if (/insufficient funds/i.test(inner)) {
    return 'Insufficient LUNC for transaction fees. Top up your wallet and try again.'
  }
  if (/out of gas/i.test(inner)) {
    return 'Transaction needed more gas than estimated. Try again — gas usage can vary slightly between blocks.'
  }
  if (/assert_deadline|deadline exceeded/i.test(inner)) {
    return 'Transaction took too long to confirm and the deadline was reached. Try again.'
  }
  if (/InvariantViolation/i.test(inner)) {
    return 'Pool state inconsistency detected. Refresh the page and try again. If this keeps happening, the pool may need operator attention.'
  }
  return null
}
