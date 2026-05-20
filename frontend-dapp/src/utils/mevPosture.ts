/**
 * Retail copy for swap / trade execution posture (public mempool, no MEV relay).
 * GitLab #168 — W10-C3.
 */

export const MEV_POSTURE_DOCS_ANCHOR = 'swap-mev-posture'

/** Relative path from the dApp origin (served or linked in dev). */
export const MEV_POSTURE_DOCS_PATH = `/docs/frontend.md#${MEV_POSTURE_DOCS_ANCHOR}`

export const MEV_POSTURE_HEADING = 'Transaction submission (MEV)'

export const MEV_POSTURE_SUMMARY =
  'Swaps are signed in your wallet and broadcast to the public Terra Classic mempool. This app does not offer a private RPC, bundle relay, or MEV-protection toggle.'

export const MEV_POSTURE_SLIPPAGE_NOTE =
  'Your slippage tolerance (max spread) is the primary on-chain guard against sandwich and front-running losses; keep it tight for large trades.'

export const MEV_POSTURE_NO_OPT_IN =
  'There is no opt-in protected submission path in this build. If that changes, this panel must gain a real control wired to the new path—not a cosmetic toggle.'
