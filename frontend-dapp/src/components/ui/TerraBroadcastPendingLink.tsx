import type { TerraBroadcastPhase } from '@/services/terraclassic/terraBroadcast'
import { TERRA_TX_RECOVERY_UNKNOWN_MESSAGE } from '@/services/terraclassic/terraTxRecoveryPoll'
import { getExplorerTxUrl, shortenTxHashForDisplay } from '@/utils/terraExplorer'

export interface TerraBroadcastPendingLinkProps {
  phase: TerraBroadcastPhase | null
  txHash: string | null
  className?: string
}

/** Explorer link shown while {@link broadcastTerraExecuteContracts} polls on-chain confirmation (GitLab #305). */
export function TerraBroadcastPendingLink({ phase, txHash, className }: TerraBroadcastPendingLinkProps) {
  if (!txHash || (phase !== 'confirming' && phase !== 'recovering')) return null

  const explorerUrl = getExplorerTxUrl(txHash)
  const label = shortenTxHashForDisplay(txHash)

  return (
    <div className={className ?? 'text-[10px] font-mono break-all'} style={{ color: 'var(--ink-dim)' }}>
      {phase === 'recovering' ? (
        <p data-testid="terra-broadcast-recovery-status">{TERRA_TX_RECOVERY_UNKNOWN_MESSAGE}</p>
      ) : null}
      <p>
        TX:{' '}
        {explorerUrl ? (
          <a
            href={explorerUrl}
            target="_blank"
            rel="noopener noreferrer"
            title={txHash}
            className="underline"
            data-testid="terra-broadcast-pending-tx"
          >
            {label}
          </a>
        ) : (
          <span title={txHash} data-testid="terra-broadcast-pending-tx">
            {label}
          </span>
        )}
      </p>
    </div>
  )
}
