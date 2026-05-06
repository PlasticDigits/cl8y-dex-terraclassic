/**
 * Map noisy chain / LCD errors to short retail copy before surfacing in the UI.
 * Raw logs stay in `console.error` upstream ([GitLab #134](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134)).
 */

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
  if (/Max spread assertion/i.test(inner)) {
    return (
      'Trade rejected: price impact exceeds your slippage tolerance on at least one pool hop. ' +
      'Try a smaller amount, pick a deeper pool route, or raise slippage tolerance in Settings (higher slippage increases execution risk).'
    )
  }
  return null
}
