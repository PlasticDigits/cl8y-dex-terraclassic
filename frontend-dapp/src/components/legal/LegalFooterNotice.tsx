import { NFA_SHORT } from '@/components/legal/legalCopy'

export default function LegalFooterNotice() {
  return (
    <p className="app-legal-footer-notice">
      {NFA_SHORT} Do your own research. By using this interface you accept all risks of interacting with on-chain
      protocols.
    </p>
  )
}
