import type { ReactNode } from 'react'
import { TermsGate } from '@plasticdigits/cl8y-clickwrap/react'
import { useWalletStore } from '@/hooks/useWallet'
import LegalKeplrInAppHint from '@/components/legal/LegalKeplrInAppHint'
import {
  getLegalClickwrapClient,
  getLegalProperty,
  resolveLegalRedirectUri,
  skipLegalClickwrapForAutomation,
} from '@/utils/legalClickwrap'

/**
 * Shell gate for wallet-bound CL8Y Legal acceptances (GitLab #517).
 *
 * Sequence: anonymous risk ack (#138) → browse → connect → this gate before route content.
 * Disconnected users keep browse access. Fail closed when status is unknown/error after connect.
 */
export default function ConnectedTermsGate({ children }: { children: ReactNode }) {
  const address = useWalletStore((s) => s.address)

  if (skipLegalClickwrapForAutomation() || !address) {
    return <>{children}</>
  }

  const redirectUri = resolveLegalRedirectUri() ?? undefined
  const property = getLegalProperty()

  return (
    <div data-testid="connected-terms-gate" className="app-connected-terms-gate">
      <LegalKeplrInAppHint address={address} />
      <TermsGate
        client={getLegalClickwrapClient()}
        property={property}
        network="TerraClassic"
        account={address}
        redirectUri={redirectUri}
        appName="CL8Y DEX"
        fallback={
          <div className="app-connected-terms-panel" role="status" aria-live="polite">
            <p className="app-connected-terms-lead">Checking terms acceptance…</p>
          </div>
        }
      >
        {children}
      </TermsGate>
    </div>
  )
}
