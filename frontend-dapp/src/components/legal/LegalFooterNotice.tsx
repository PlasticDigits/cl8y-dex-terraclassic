import { NFA_SHORT, SECURITY_REPORT_ISSUE_URL, USER_INCIDENT_FAQ_HREF } from '@/components/legal/legalCopy'
import { SECURITY_POSTURE_DOC_URL } from '@/utils/constants'
import { POOL_LP_HOWTO_FOOTER_LABEL, POOL_LP_HOWTO_HREF } from '@/utils/poolLpHowtoCopy'

export default function LegalFooterNotice() {
  return (
    <p className="app-legal-footer-notice">
      {NFA_SHORT} DYOR.{' '}
      <a
        className="underline hover:opacity-80"
        href={SECURITY_POSTURE_DOC_URL}
        target="_blank"
        rel="noopener noreferrer"
        data-testid="security-posture-doc-link"
      >
        Security
      </a>
      {' · '}
      <a
        href={USER_INCIDENT_FAQ_HREF}
        target="_blank"
        rel="noopener noreferrer"
        className="underline"
        data-testid="user-incident-faq-link"
      >
        Incidents
      </a>
      {' · '}
      <a href={POOL_LP_HOWTO_HREF} className="underline" data-testid="pool-lp-howto-footer-link">
        {POOL_LP_HOWTO_FOOTER_LABEL}
      </a>
      {' · '}
      <a className="app-legal-footer-link" href={SECURITY_REPORT_ISSUE_URL} target="_blank" rel="noopener noreferrer">
        Report
      </a>
      .
    </p>
  )
}
