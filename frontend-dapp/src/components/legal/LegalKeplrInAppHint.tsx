import { useEffect, useState } from 'react'
import { useWalletStore } from '@/hooks/useWallet'
import { getLegalClickwrapClient, getLegalProperty } from '@/utils/legalClickwrap'
import {
  hasLegalSignerInjector,
  legalTermsWalletHint,
  shouldShowLegalWalletInAppHint,
} from '@/utils/legalKeplrInAppHint'

/**
 * Next-step copy when Legal Accept still needs a signer injector (GitLab #554 / #658).
 * Does not implement ADR-036 (C1). Plain text — not a wallet-download link.
 */
export default function LegalKeplrInAppHint({ address }: { address: string }) {
  const walletType = useWalletStore((s) => s.walletType)
  const [signedLatest, setSignedLatest] = useState<boolean | null>(null)

  useEffect(() => {
    let cancelled = false
    getLegalClickwrapClient()
      .getSignatureStatus(getLegalProperty(), 'TERRA_CLASSIC', address)
      .then((status) => {
        if (!cancelled) setSignedLatest(Boolean(status?.signed_latest))
      })
      .catch(() => {
        if (!cancelled) setSignedLatest(null)
      })
    return () => {
      cancelled = true
    }
  }, [address])

  const hasSignerInjector = hasLegalSignerInjector()
  if (!shouldShowLegalWalletInAppHint({ hasSignerInjector, signedLatest })) {
    return null
  }

  return (
    <p data-testid="legal-wallet-inapp-hint" className="app-connected-terms-wallet-hint">
      {legalTermsWalletHint(walletType)}
    </p>
  )
}
