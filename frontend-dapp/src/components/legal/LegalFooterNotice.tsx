import {
  NFA_SHORT,
  SECURITY_REPORT_ISSUE_URL,
  USER_INCIDENT_FAQ_HREF,
  USER_INCIDENT_FAQ_LABEL,
} from '@/components/legal/legalCopy'

export default function LegalFooterNotice() {
  return (
    <p className="app-legal-footer-notice">
      {NFA_SHORT} Do your own research. By using this interface you accept all risks of interacting with on-chain
      protocols.{' '}
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
