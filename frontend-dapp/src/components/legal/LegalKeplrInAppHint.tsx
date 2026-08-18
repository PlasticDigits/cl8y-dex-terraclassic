import { useEffect, useState } from 'react'
import { WalletName } from '@goblinhunt/cosmes/wallet'
import { isBrowserWalletExtensionDetected } from '@/services/terraclassic/walletExtensionInstall'
import { getLegalClickwrapClient, getLegalProperty } from '@/utils/legalClickwrap'
import { LEGAL_KEPLR_INAPP_HINT, shouldShowLegalKeplrInAppHint } from '@/utils/legalKeplrInAppHint'

/**
 * Next-step copy when Legal Accept still needs `window.keplr` (GitLab #554).
 * Does not implement ADR-036 (C1).
 */
export default function LegalKeplrInAppHint({ address }: { address: string }) {
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

  const hasKeplrExtension = isBrowserWalletExtensionDetected(WalletName.KEPLR)
  if (!shouldShowLegalKeplrInAppHint({ hasKeplrExtension, signedLatest })) {
    return null
  }

  return (
    <p data-testid="legal-keplr-inapp-hint" className="app-connected-terms-keplr-hint">
      {LEGAL_KEPLR_INAPP_HINT}
    </p>
  )
}
