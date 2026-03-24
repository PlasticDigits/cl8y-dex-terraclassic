import { getExplorerTxUrl } from '@/utils/terraExplorer'

export interface TxResultAlertProps {
  type: 'success' | 'error'
  message: string
  txHash?: string
}

export function TxResultAlert({ type, message, txHash }: TxResultAlertProps) {
  const baseClass = type === 'success' ? 'alert-success' : 'alert-error'
  const explorerUrl = txHash ? getExplorerTxUrl(txHash) : null

  return (
    <div className={baseClass}>
      {message}
      {type === 'success' && txHash != null && (
        <>
          {' '}
          TX:{' '}
          {explorerUrl ? (
            <a
              href={explorerUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="font-mono text-xs underline hover:opacity-80"
            >
              {txHash.slice(0, 12)}...{txHash.slice(-6)}
            </a>
          ) : (
            <span className="font-mono text-xs">{txHash}</span>
          )}
        </>
      )}
    </div>
  )
}
