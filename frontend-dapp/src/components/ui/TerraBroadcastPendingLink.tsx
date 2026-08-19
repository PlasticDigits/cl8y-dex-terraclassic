import { useEffect, useState } from 'react'
import type { TerraBroadcastPhase } from '@/services/terraclassic/terraBroadcast'
import { getSessionNanoLedger } from '@/services/terraclassic/keplrExtensionConfig'
import { terraBroadcastPendingStatusMessage } from '@/utils/terraBroadcastUi'
import { getExplorerTxUrl, shortenTxHashForDisplay } from '@/utils/terraExplorer'
import { TERRA_TX_SIGNING_HINT_DELAY_MS } from '@/utils/terraTxTimeout'

export interface TerraBroadcastPendingLinkProps {
  phase: TerraBroadcastPhase | null
  txHash: string | null
  className?: string
  /** Override Ledger detection (tests). Defaults to the connected Keplr session flag. */
  isNanoLedger?: boolean
}

/** Explorer link + post-sign recovery copy during broadcast confirmation (GitLab #305, #359, #567). */
export function TerraBroadcastPendingLink({ phase, txHash, className, isNanoLedger }: TerraBroadcastPendingLinkProps) {
  const resolvedNano = isNanoLedger ?? getSessionNanoLedger()
  const [signingElapsedMs, setSigningElapsedMs] = useState(0)

  useEffect(() => {
    if (phase !== 'signing' || resolvedNano) {
      setSigningElapsedMs(0)
      return
    }
    setSigningElapsedMs(0)
    const timeout = window.setTimeout(() => {
      setSigningElapsedMs(TERRA_TX_SIGNING_HINT_DELAY_MS)
    }, TERRA_TX_SIGNING_HINT_DELAY_MS)
    return () => {
      window.clearTimeout(timeout)
    }
  }, [phase, resolvedNano])

  const statusMessage = terraBroadcastPendingStatusMessage(phase, {
    isNanoLedger: resolvedNano,
    signingElapsedMs,
  })
  const showTxLink = (phase === 'confirming' || phase === 'recovering') && txHash

  if (!statusMessage && !showTxLink) return null

  const explorerUrl = txHash ? getExplorerTxUrl(txHash) : null
  const label = txHash ? shortenTxHashForDisplay(txHash) : ''
  const signingHint = phase === 'signing' && statusMessage

  return (
    <div className={className ?? 'text-[10px] break-all'} style={{ color: 'var(--ink-dim)' }}>
      {statusMessage ? (
        <p
          className="mb-1"
          data-testid={signingHint ? 'terra-broadcast-signing-hint' : 'terra-broadcast-recovery-status'}
        >
          {statusMessage}
        </p>
      ) : null}
      {showTxLink ? (
        <p className="font-mono">
          TX:{' '}
          {explorerUrl ? (
            <a
              href={explorerUrl}
              target="_blank"
              rel="noopener noreferrer"
              title={txHash!}
              className="underline"
              data-testid="terra-broadcast-pending-tx"
            >
              {label}
            </a>
          ) : (
            <span title={txHash!} data-testid="terra-broadcast-pending-tx">
              {label}
            </span>
          )}
        </p>
      ) : null}
    </div>
  )
}
