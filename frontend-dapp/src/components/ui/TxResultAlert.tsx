import { getExplorerTxUrl, shortenTxHashForDisplay } from '@/utils/terraExplorer'

export interface TxResultAlertProps {
  type: 'success' | 'error'
  message: string
  txHash?: string
}

export function TxResultAlert({ type, message, txHash }: TxResultAlertProps) {
  const baseClass = type === 'success' ? 'alert-success' : 'alert-error'
  const explorerUrl = txHash ? getExplorerTxUrl(txHash) : null
  const txLabel = txHash ? shortenTxHashForDisplay(txHash) : ''

  return (
    <div className={`${baseClass} min-w-0 max-w-full break-words`}>
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
              title={txHash}
              className="font-mono text-xs underline hover:opacity-80 break-all"
            >
              {txLabel}
            </a>
          ) : (
            <span className="font-mono text-xs break-all" title={txHash}>
              {txLabel}
            </span>
          )}
        </>
      )}
    </div>
  )
}
