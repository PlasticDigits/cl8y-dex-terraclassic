import type { TerraBroadcastPhase } from '@/services/terraclassic/terraBroadcast'
import { getExplorerTxUrl, shortenTxHashForDisplay } from '@/utils/terraExplorer'

export interface TerraBroadcastPendingLinkProps {
  phase: TerraBroadcastPhase | null
  txHash: string | null
  className?: string
}

/** Explorer link shown while {@link broadcastTerraExecuteContracts} polls on-chain confirmation (GitLab #305). */
export function TerraBroadcastPendingLink({ phase, txHash, className }: TerraBroadcastPendingLinkProps) {
  if (phase !== 'confirming' || !txHash) return null

  const explorerUrl = getExplorerTxUrl(txHash)
  const label = shortenTxHashForDisplay(txHash)

  return (
    <p className={className ?? 'text-[10px] font-mono break-all'} style={{ color: 'var(--ink-dim)' }}>
      TX:{' '}
      {explorerUrl ? (
        <a href={explorerUrl} target="_blank" rel="noopener noreferrer" title={txHash} className="underline">
          {label}
        </a>
      ) : (
        <span title={txHash}>{label}</span>
      )}
    </p>
  )
}
