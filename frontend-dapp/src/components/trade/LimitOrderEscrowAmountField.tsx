import { useId, useMemo } from 'react'
import type { UseQueryResult } from '@tanstack/react-query'
import { AmountBalanceActions } from '@/components/common/AmountBalanceActions'
import {
  computeMaxSpendableHumanAmount,
  type MaxAmountContext,
  type NativeSwapMaxHints,
} from '@/utils/maxSpendableAmount'

type Props = {
  escrowLabel: string
  escrowDecimals: number
  amountHuman: string
  onAmountChange: (v: string) => void
  balanceQuery: UseQueryResult<string, Error>
  /** When balance loads, set amount to computed max. */
  onMax: (human: string) => void
  walletConnected: boolean
  compact?: boolean
  maxContext: MaxAmountContext
  assetIsNativeUluna?: boolean
  nativeSwapHints?: NativeSwapMaxHints
  marketUsesHybrid?: boolean
  limitPlaceRungCount?: number
  /**
   * When set (limit place only), show a headline-scaled USD line under the input when the amount is non-empty.
   * `null` means headline/ref unavailable — display an em dash (same coverage as limit price USD anchor; GitLab #155).
   */
  escrowUsdNotionalApprox?: string | null
}

/**
 * Escrow amount field with shared balance + Max row ([GitLab #213](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/213)).
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
  maxContext,
  assetIsNativeUluna = false,
  nativeSwapHints,
  marketUsesHybrid,
  limitPlaceRungCount,
  escrowUsdNotionalApprox,
}: Props) {
  const amountInputId = useId()
  const showUsd = escrowUsdNotionalApprox !== undefined && amountHuman.trim() !== ''

  const maxResult = useMemo(() => {
    if (!balanceQuery.data) {
      return { human: '0', spendableRaw: 0n, cappedByGas: false, reserveUluna: 0n }
    }
    return computeMaxSpendableHumanAmount({
      balanceRaw: balanceQuery.data,
      decimals: escrowDecimals,
      assetIsNativeUluna,
      context: maxContext,
      nativeSwapHints,
      marketUsesHybrid,
      limitPlaceRungCount,
    })
  }, [
    balanceQuery.data,
    escrowDecimals,
    assetIsNativeUluna,
    maxContext,
    nativeSwapHints,
    marketUsesHybrid,
    limitPlaceRungCount,
  ])

  return (
    <div>
      <label className={compact ? 'label-neo text-[10px]' : 'label-neo'} htmlFor={amountInputId}>
        Amount ({escrowLabel})
      </label>
      <input
        id={amountInputId}
        className={compact ? 'input-neo w-full text-sm' : 'input-neo w-full'}
        value={amountHuman}
        onChange={(e) => onAmountChange(e.target.value)}
        placeholder="0.0"
        data-testid="limit-order-escrow-amount-input"
      />
      {showUsd && (
        <p
          className={(compact ? 'text-[10px] ' : 'text-xs ') + 'mt-1 tabular-nums'}
          style={{ color: 'var(--ink-dim)' }}
          data-testid="limit-order-escrow-usd-notional"
        >
          <span className="font-medium" style={{ color: 'var(--ink-subtle)' }}>
            Headline USD (escrow):{' '}
          </span>
          {escrowUsdNotionalApprox != null ? <span>≈ {escrowUsdNotionalApprox}</span> : <span>—</span>}
        </p>
      )}
      <AmountBalanceActions
        balanceQuery={balanceQuery}
        decimals={escrowDecimals}
        walletConnected={walletConnected}
        compact={compact}
        spendableRaw={maxResult.spendableRaw}
        onMax={() => onMax(maxResult.human)}
        testIdMax="limit-order-escrow-max"
      />
    </div>
  )
}
