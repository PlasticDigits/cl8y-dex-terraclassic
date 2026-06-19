import { NFA_SHORT, USER_INCIDENT_FAQ_HREF, USER_INCIDENT_FAQ_LABEL } from '@/components/legal/legalCopy'

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
    </p>
  )
}
