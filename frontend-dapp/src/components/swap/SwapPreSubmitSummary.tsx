import type { ReactNode } from 'react'
import { AddressRow } from '@/components/ui/AddressRow'
import { getNetworkBadgeCopy } from '@/utils/networkDisplay'
import { SLIPPAGE_PROTECTION_LABEL } from '@/utils/slippageProtectionCopy'

export type SwapPreSubmitSummaryProps = {
  /** Wallet action shown before extension signing (default: Swap). */
  actionLabel?: string
  offerSymbol: string
  receiveSymbol: string
  offerAmountHuman: string
  receiveAmountHuman: string
  /** Slippage protection passed on-chain as max_spread (percent). */
  maxSpreadPercent: number
  /** Minimum receive after slippage floor; null while quote is incomplete. */
  minReceiveHuman: string | null
  /** Factory-sourced pair contract address(es) for the submit route ([#449](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/449)). */
  pairContractAddresses?: string[]
  /** Active Terra Classic network label (defaults to env badge copy). */
  chainFullLabel?: string
  'data-testid'?: string
}

function SummaryRow({ label, value, testId }: { label: string; value: ReactNode; testId: string }) {
  return (
    <div className="flex flex-col gap-0.5 sm:flex-row sm:items-start sm:justify-between" data-testid={testId}>
      <span className="uppercase text-[10px] tracking-wide font-medium shrink-0" style={{ color: 'var(--ink-subtle)' }}>
        {label}
      </span>
      <span
        className="font-mono text-[10px] sm:text-right break-words min-w-0 sm:max-w-[65%]"
        style={{ color: 'var(--ink)' }}
      >
        {value}
      </span>
    </div>
  )
}

function PairContractAddressesValue({ addresses }: { addresses: string[] }) {
  if (addresses.length === 1) {
    return (
      <AddressRow
        address={addresses[0]!}
        startChars={8}
        endChars={6}
        copyAriaLabel="Copy pair contract address"
        explorerAriaLabel="View pair contract on explorer"
        data-testid="swap-confirm-pair-contract"
      />
    )
  }

  return (
    <span className="inline-flex flex-wrap items-center justify-end gap-x-1 gap-y-0.5">
      {addresses.map((addr, index) => (
        <span key={addr} className="inline-flex items-center gap-x-1">
          {index > 0 ? <span aria-hidden="true">→</span> : null}
          <AddressRow
            address={addr}
            startChars={6}
            endChars={4}
            copyAriaLabel={`Copy hop ${index + 1} pair contract address`}
            explorerAriaLabel={`View hop ${index + 1} pair contract on explorer`}
            data-testid={`swap-confirm-hop-pair-${index}`}
          />
        </span>
      ))}
    </span>
  )
}

/**
 * Pre-sign summary for taker swaps: labeled action, pair, amounts, slippage protection, min return, and chain
 * so wallet phishing cannot substitute pair/chain without failing tests ([#409](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/409) / SEC-D11).
 * Factory-sourced pair contract address(es) are shown for transparency ([#449](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/449)).
 */
export function SwapPreSubmitSummary({
  actionLabel = 'Swap',
  offerSymbol,
  receiveSymbol,
  offerAmountHuman,
  receiveAmountHuman,
  maxSpreadPercent,
  minReceiveHuman,
  pairContractAddresses = [],
  chainFullLabel = getNetworkBadgeCopy().fullLabel,
  'data-testid': testId = 'swap-pre-submit-summary',
}: SwapPreSubmitSummaryProps) {
  const pairLabel = `${offerSymbol} → ${receiveSymbol}`
  const pairContractLabel = pairContractAddresses.length === 1 ? 'Pair contract' : 'Pair contracts'

  return (
    <div
      className="card-glass !p-2.5 space-y-2 text-[11px] sm:text-xs mb-3"
      data-testid={testId}
      role="region"
      aria-label="Swap summary before signing"
    >
      <div className="space-y-1.5">
        <SummaryRow label="Action" value={actionLabel} testId="swap-confirm-action" />
        <SummaryRow label="Pair" value={pairLabel} testId="swap-confirm-pair" />
        {pairContractAddresses.length > 0 ? (
          <SummaryRow
            label={pairContractLabel}
            value={<PairContractAddressesValue addresses={pairContractAddresses} />}
            testId="swap-confirm-pair-contracts"
          />
        ) : null}
        <SummaryRow label="You pay" value={`${offerAmountHuman} ${offerSymbol}`} testId="swap-confirm-offer" />
        <SummaryRow
          label="You receive (est.)"
          value={`${receiveAmountHuman} ${receiveSymbol}`}
          testId="swap-confirm-receive"
        />
        <SummaryRow label={SLIPPAGE_PROTECTION_LABEL} value={`${maxSpreadPercent}%`} testId="swap-confirm-max-spread" />
        <SummaryRow
          label="Min return"
          value={minReceiveHuman != null ? `${minReceiveHuman} ${receiveSymbol}` : '—'}
          testId="swap-confirm-min-return"
        />
        <SummaryRow label="Chain" value={chainFullLabel} testId="swap-confirm-chain" />
      </div>
    </div>
  )
}
