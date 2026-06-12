import type { TerraBroadcastPhase } from '@/services/terraclassic/terraBroadcast'
import { terraBroadcastPendingStatusMessage } from '@/utils/terraBroadcastUi'
import { getExplorerTxUrl, shortenTxHashForDisplay } from '@/utils/terraExplorer'

export interface TerraBroadcastPendingLinkProps {
  phase: TerraBroadcastPhase | null
  txHash: string | null
  className?: string
}

/** Explorer link + post-sign recovery copy during broadcast confirmation (GitLab #305, #359). */
export function TerraBroadcastPendingLink({ phase, txHash, className }: TerraBroadcastPendingLinkProps) {
  const statusMessage = terraBroadcastPendingStatusMessage(phase)
  const showTxLink = (phase === 'confirming' || phase === 'recovering') && txHash

  if (!statusMessage && !showTxLink) return null

  const explorerUrl = txHash ? getExplorerTxUrl(txHash) : null
  const label = txHash ? shortenTxHashForDisplay(txHash) : ''

  return (
    <div className={className ?? 'text-[10px] break-all'} style={{ color: 'var(--ink-dim)' }}>
      {statusMessage ? (
        <p className="mb-1" data-testid="terra-broadcast-recovery-status">
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
