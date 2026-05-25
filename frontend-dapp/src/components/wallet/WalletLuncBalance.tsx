import { useNativeUlunaBalance } from '@/hooks/useNativeUlunaBalance'
import { formatTokenAmount } from '@/utils/formatAmount'
import { Spinner } from '@/components/ui'

const LUNC_DECIMALS = 6

type WalletLuncBalanceProps = {
  address: string
  className?: string
}

/**
 * Bank uluna (LUNC) balance for the connected wallet chip and menu header.
 * Shares React Query cache with swap/pool/trade via `useNativeUlunaBalance`.
 */
export function WalletLuncBalance({ address, className = '' }: WalletLuncBalanceProps) {
  const query = useNativeUlunaBalance(address)

  if (query.isLoading) {
    return (
      <span
        className={`inline-flex items-center gap-1 min-h-[1rem] ${className}`}
        aria-busy="true"
        data-testid="wallet-lunc-balance"
      >
        <Spinner size="sm" className="!w-3 !h-3 opacity-90" />
        <span className="sr-only">Loading LUNC balance</span>
      </span>
    )
  }

  if (query.isError) {
    return (
      <span
        className={`font-mono text-xs tabular-nums ${className}`}
        data-testid="wallet-lunc-balance"
        title="Could not load LUNC balance"
      >
        — LUNC
      </span>
    )
  }

  const human = formatTokenAmount(query.data ?? '0', LUNC_DECIMALS, 4)
  return (
    <span
      className={`font-mono text-xs font-semibold tabular-nums truncate ${className}`}
      data-testid="wallet-lunc-balance"
      title={`${human} LUNC`}
      style={{ color: 'var(--ink)' }}
    >
      {human} LUNC
    </span>
  )
}
