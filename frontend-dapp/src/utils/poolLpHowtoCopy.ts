/** Retail LUNC liquidity how-to copy (GitLab #531 / #533). Static strings only — no indexer/wallet interpolation. */

import { ENVIRONMENT_EXPLAINER } from '@/components/legal/legalCopy'

export const POOL_LP_HOWTO_ANCHOR = 'lp-howto'
export const POOL_LP_HOWTO_HREF = `/pool#${POOL_LP_HOWTO_ANCHOR}`
export const POOL_LP_HOWTO_HINT = 'Add one token. Native LUNC auto-wraps.'
export const POOL_LP_HOWTO_SUMMARY = 'How to add liquidity'
export const POOL_LP_HOWTO_OPEN_LABEL = 'How to'
export const POOL_LP_HOWTO_DISMISS_LABEL = 'Dismiss'
export const POOL_LP_HOWTO_FOOTER_LABEL = 'Add liquidity'

export const POOL_LP_HOWTO_NO_INCENTIVE = 'There is no LP or maker incentive program currently.'

export const POOL_LP_HOWTO_TWO_SIDED =
  'Default Add uses one token you already hold. The other side is swapped to the pool ratio automatically. Two-sided deposit is Advanced (empty pools).'

export const POOL_LP_HOWTO_WRAP =
  'Pools hold wrapped LUNC (cLUNC). Pick native LUNC as Token to auto-wrap, or wrap first under More → Wrap.'

export const POOL_LP_HOWTO_GAS =
  'Keep bank LUNC for gas. One-sided add can wrap, swap, and provide in one signing flow — do not skip the gas check.'

export const POOL_LP_HOWTO_WITHDRAW =
  'LP tokens are your pool share. To withdraw: pick LP, Withdraw as one token, enter the amount, review the preview, and sign.'

export const POOL_LP_HOWTO_LIMITS =
  'Optional: place a limit on Trade or Limits. That escrows one token at a price — it is not a pool share and not a rewards program.'

export const POOL_LP_HOWTO_CREATE_PAIR_FEE =
  'Create Pair (under More) is only for a new market. That page charges a LUNC creation fee plus gas. You do not need it to add to an existing pool.'

export const POOL_LP_HOWTO_RATIO_DONATE =
  'Retail Add swaps to the pool ratio so extra is not donated. Advanced two-sided can still donate if you type off-ratio amounts.'

export const POOL_LP_HOWTO_UNWRAP =
  'Unwrapping is not free LUNC out. Use Wrap for fee quotes, and do not send unwrapped LUNC to an exchange without the wrap-page warning.'

export const POOL_LP_HOWTO_NETWORK = ENVIRONMENT_EXPLAINER

export const POOL_LP_HOWTO_PROVIDE =
  'Open Pool (header, More on a tablet, or the phone tab). Pick Token, Pair, and Amount → read the impermanent-loss notice and pre-sign summary → Add.'

export type PoolLpHowtoLink = {
  readonly href: '/wrap' | '/trade' | '/limits' | '/create'
  readonly label: string
  readonly testId: string
}

/** In-app destinations only (A2). No third-party URLs or deposit addresses. */
export const POOL_LP_HOWTO_LINKS: readonly PoolLpHowtoLink[] = [
  { href: '/wrap', label: 'Wrap', testId: 'pool-lp-howto-wrap-link' },
  { href: '/trade', label: 'Trade', testId: 'pool-lp-howto-trade-link' },
  { href: '/limits', label: 'Limits', testId: 'pool-lp-howto-limits-link' },
  { href: '/create', label: 'Create Pair', testId: 'pool-lp-howto-create-link' },
]

export const POOL_LP_HOWTO_STEPS: readonly { readonly id: string; readonly text: string }[] = [
  { id: 'network', text: POOL_LP_HOWTO_NETWORK },
  { id: 'provide', text: POOL_LP_HOWTO_PROVIDE },
  { id: 'two-sided', text: POOL_LP_HOWTO_TWO_SIDED },
  { id: 'wrap', text: POOL_LP_HOWTO_WRAP },
  { id: 'gas', text: POOL_LP_HOWTO_GAS },
  { id: 'withdraw', text: POOL_LP_HOWTO_WITHDRAW },
  { id: 'ratio', text: POOL_LP_HOWTO_RATIO_DONATE },
  { id: 'create-fee', text: POOL_LP_HOWTO_CREATE_PAIR_FEE },
  { id: 'limits', text: POOL_LP_HOWTO_LIMITS },
  { id: 'no-incentive', text: POOL_LP_HOWTO_NO_INCENTIVE },
  { id: 'unwrap', text: POOL_LP_HOWTO_UNWRAP },
]

const FORBIDDEN_COPY = [
  /\btoken0\b/i,
  /\btoken1\b/i,
  /\bAPR\b/i,
  /\bAPY\b/i,
  /\bpoints?\b/i,
  /\breward(s|ed)?\s+api/i,
  /\bfarm(ing)?\b/i,
  /\bairdrop\b/i,
  /\bmnemonic\b/i,
  /\bCW20 Send\b/i,
  /\bAMM invariant\b/i,
  /send\s+lunc\s+to\s+this\s+address/i,
]

export function poolLpHowtoAllText(): string {
  return [
    POOL_LP_HOWTO_HINT,
    POOL_LP_HOWTO_SUMMARY,
    ...POOL_LP_HOWTO_STEPS.map((step) => step.text),
    ...POOL_LP_HOWTO_LINKS.map((link) => `${link.href} ${link.label}`),
  ].join('\n')
}

/** Returns matching forbidden patterns (empty = copy is safe for A2 / A10 / #489). */
export function forbiddenPoolLpHowtoCopyHits(text: string = poolLpHowtoAllText()): string[] {
  return FORBIDDEN_COPY.filter((re) => re.test(text)).map((re) => String(re))
}
