/**
 * F6 listed-CW20 code-id freeze (GitLab #585).
 * On-chain execute fails closed; LCD quotes stay ungated. UI must say so.
 */

export const CODE_ID_FROZEN_CTA = 'Market frozen'

export const CODE_ID_FROZEN_BANNER =
  'This market is frozen because a listed token’s code changed. Quotes can still appear, but swaps, LP, and limit actions are blocked until operators restore the market.'

export const CODE_ID_FROZEN_TX_MESSAGE =
  'This market is frozen because a listed token’s code changed. Trading is blocked until operators restore it.'

export const CODE_ID_GUARD_UNAVAILABLE_TX_MESSAGE = 'This market could not verify token code. Try again later.'

export const CODE_ID_UNPINNED_TX_MESSAGE =
  'This market is not yet upgraded for token-code checks. If a trade fails, try another pair.'

export type CodeIdFreezeVerdict = 'tradable' | 'frozen'

/** Pure pin vs live + whitelist check. Both legs must pass. */
export function evaluateLivePins(input: {
  pin0: number
  pin1: number
  live0: number
  live1: number
  whitelisted0: boolean
  whitelisted1: boolean
}): CodeIdFreezeVerdict {
  if (input.live0 !== input.pin0 || input.live1 !== input.pin1 || !input.whitelisted0 || !input.whitelisted1) {
    return 'frozen'
  }
  return 'tradable'
}

export function isPreF6AssetCodeIdsError(message: string): boolean {
  const s = message.toLowerCase()
  return (
    s.includes('unknown variant') ||
    s.includes('error parsing into type') ||
    s.includes('pins are missing') ||
    s.includes('assetcodeidunpinned')
  )
}
