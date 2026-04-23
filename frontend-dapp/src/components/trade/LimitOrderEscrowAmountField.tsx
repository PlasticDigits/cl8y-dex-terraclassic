import type { UseQueryResult } from '@tanstack/react-query'
import { sounds } from '@/lib/sounds'
import { Spinner } from '@/components/ui'
import { formatTokenAmount, fromRawAmount } from '@/utils/formatAmount'

type Props = {
  escrowLabel: string
  escrowDecimals: number
  amountHuman: string
  onAmountChange: (v: string) => void
  balanceQuery: UseQueryResult<string, Error>
  /** When balance loads, set amount to full raw balance. */
  onMax: (human: string) => void
  walletConnected: boolean
  compact?: boolean
}

/**
 * Same pattern as the swap “You Pay” card: show escrow balance + Max.
 */
export function LimitOrderEscrowAmountField({
  escrowLabel,
  escrowDecimals,
  amountHuman,
  onAmountChange,
  balanceQuery,
  onMax,
  walletConnected,
  compact,
}: Props) {
  return (
    <div>
      <label className={compact ? 'label-neo text-[10px]' : 'label-neo'}>Amount ({escrowLabel})</label>
      <input
        className={compact ? 'input-neo w-full text-sm' : 'input-neo w-full'}
        value={amountHuman}
        onChange={(e) => onAmountChange(e.target.value)}
        placeholder="0.0"
      />
      {walletConnected && (
        <div
          className={
            (compact ? 'text-[10px] ' : 'text-xs ') +
            'flex flex-wrap items-center justify-between gap-2 mt-1.5 min-h-[1.25rem]'
          }
          style={{ color: 'var(--ink-subtle)' }}
        >
          <span className="inline-flex items-center gap-1.5 min-w-0 max-w-full">
            <span className="shrink-0">Balance:</span>
            {balanceQuery.isLoading ? (
              <span className="inline-flex items-center" aria-busy="true" aria-live="polite">
                <Spinner size="sm" className="!w-3.5 !h-3.5 opacity-90" />
                <span className="sr-only">Loading balance</span>
              </span>
            ) : balanceQuery.isError ? (
              <span className="font-mono">—</span>
            ) : (
              <span className="font-mono truncate">{formatTokenAmount(balanceQuery.data ?? '0', escrowDecimals)}</span>
            )}
          </span>
          <button
            type="button"
            disabled={balanceQuery.isLoading || balanceQuery.isError || !balanceQuery.data}
            onClick={() => {
              sounds.playButtonPress()
              if (balanceQuery.data) onMax(fromRawAmount(balanceQuery.data, escrowDecimals))
            }}
            className="ml-auto uppercase font-semibold tracking-wide hover:underline shrink-0 disabled:opacity-40 disabled:cursor-not-allowed disabled:no-underline"
            style={{ color: 'var(--cyan)' }}
          >
            Max
          </button>
        </div>
      )}
    </div>
  )
}
