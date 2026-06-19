/** Shared legal / risk copy for modal and footer (GitLab #138). */

import { DOCS_GITLAB_BASE } from '@/utils/constants'

export const USER_INCIDENT_FAQ_HREF = `${DOCS_GITLAB_BASE}/user-incident-faq.md`
export const USER_INCIDENT_FAQ_LABEL = 'What happens during an incident?'

export const NFA_SHORT =
  'CL8Y DEX is experimental software. Nothing here is financial, investment, legal, or tax advice (NFA).'

export const RISK_SUMMARY_BULLETS: readonly string[] = [
  'Digital assets are volatile; you may lose some or all funds you deposit or trade.',
  'Smart contracts and bridges carry technical and operational risk, including bugs and exploits.',
  'You are solely responsible for your wallet, keys, and compliance with laws that apply to you.',
]

export const ENVIRONMENT_EXPLAINER =
  'Always verify the network badge and the environment strip below the header before signing transactions.'
