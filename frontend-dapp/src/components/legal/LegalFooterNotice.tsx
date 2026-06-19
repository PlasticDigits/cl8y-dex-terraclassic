import {
  NFA_SHORT,
  SECURITY_REPORT_ISSUE_URL,
  USER_INCIDENT_FAQ_HREF,
  USER_INCIDENT_FAQ_LABEL,
} from '@/components/legal/legalCopy'
import { SECURITY_POSTURE_DOC_URL } from '@/utils/constants'

export default function LegalFooterNotice() {
  return (
    <p className="app-legal-footer-notice">
      {NFA_SHORT} Do your own research. By using this interface you accept all risks of interacting with on-chain
      protocols.{' '}
      <a
        className="underline hover:opacity-80"
        href={SECURITY_POSTURE_DOC_URL}
        target="_blank"
        rel="noopener noreferrer"
        data-testid="security-posture-doc-link"
      >
        Security and audit docs
      </a>
      {' · '}
      <a
        href={USER_INCIDENT_FAQ_HREF}
        target="_blank"
        rel="noopener noreferrer"
        className="underline"
        data-testid="user-incident-faq-link"
      >
        {USER_INCIDENT_FAQ_LABEL}
      </a>
      {' · '}
      <a className="app-legal-footer-link" href={SECURITY_REPORT_ISSUE_URL} target="_blank" rel="noopener noreferrer">
        Report suspicious activity
      </a>
      .
    </p>
  )
}
