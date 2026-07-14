import type { ReactNode } from 'react'
import type { UseQueryResult } from '@tanstack/react-query'
import { Spinner } from '@/components/ui'
import { formatTokenAmount } from '@/utils/formatAmount'

type Props = {
  receiveLabel: string
  receiveAmountHuman: string | null
  receiveDecimals: number
  receiveBalanceQuery: UseQueryResult<string, Error>
  walletConnected: boolean
  receiveUsdNotionalApprox?: string | null
  compact?: boolean
}

/**
 * Read-only counter-asset row for limit place (#488 concept Pay → Receive stack).
 */
export function LimitOrderReceiveField({
  receiveLabel,
  receiveAmountHuman,
  receiveDecimals,
  receiveBalanceQuery,
  walletConnected,
  receiveUsdNotionalApprox,
  compact,
}: Props) {
  const showUsd = receiveUsdNotionalApprox !== undefined
  const displayAmount = receiveAmountHuman ?? '—'
  const textSize = compact ? 'text-[10px]' : 'text-xs'

  let balanceContent: ReactNode = '—'
  if (walletConnected) {
    if (receiveBalanceQuery.isLoading) {
      balanceContent = <Spinner className="inline-block w-3 h-3" />
    } else if (!receiveBalanceQuery.isError && receiveBalanceQuery.data) {
      balanceContent = formatTokenAmount(receiveBalanceQuery.data, receiveDecimals, 4)
    }
  }

  return (
    <div data-testid="limit-order-receive-field">
      <div className={`flex items-center justify-between gap-2 ${compact ? 'mb-1' : 'mb-1.5'}`}>
        <label className={compact ? 'label-glass text-[10px] !mb-0' : 'label-glass !mb-0'}>
          Receive ({receiveLabel})
        </label>
        <span className={textSize} style={{ color: 'var(--ink-subtle)' }} data-testid="limit-order-receive-balance">
          Balance: {balanceContent}
        </span>
      </div>
      <div
        className={
          (compact ? 'input-glass w-full text-sm ' : 'input-glass w-full ') +
          'font-mono tabular-nums flex items-center min-h-[2.5rem] px-3'
        }
        data-testid="limit-order-receive-amount"
        aria-live="polite"
      >
        {displayAmount}
      </div>
      {showUsd && receiveAmountHuman != null && (
        <p
          className={(compact ? 'text-[10px] ' : 'text-xs ') + 'mt-1 tabular-nums'}
          style={{ color: 'var(--ink-dim)' }}
          data-testid="limit-order-receive-usd-notional"
        >
          {receiveUsdNotionalApprox != null ? (
            <span>≈ {receiveUsdNotionalApprox}</span>
          ) : (
            <span style={{ color: 'var(--ink-subtle)' }}>—</span>
          )}
        </p>
      )}
    </div>
  )
}
