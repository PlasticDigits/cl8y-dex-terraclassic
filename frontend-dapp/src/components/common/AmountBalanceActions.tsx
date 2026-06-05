import type { UseQueryResult } from '@tanstack/react-query'
import { sounds } from '@/lib/sounds'
import { Spinner } from '@/components/ui'
import { formatTokenAmount, fromRawAmount } from '@/utils/formatAmount'

type Props = {
  balanceQuery: UseQueryResult<string, Error>
  decimals: number
  walletConnected: boolean
  compact?: boolean
  /** When false, balance row is hidden (e.g. no asset selected). */
  showBalance?: boolean
  showHalf?: boolean
  spendableRaw: bigint
  onMax: () => void
  onHalf?: () => void
  /**
   * Optional fraction preset buttons (e.g. 25/50/75/100%). When provided, one
   * button per `fractionPresets` entry is rendered; each applies its fraction to
   * the same gas-adjusted `spendableRaw` that `onMax` uses and passes the
   * resulting human draft to the caller. Backwards-compatible with the existing
   * `showHalf`/`onMax` callers ([GitLab #303](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/303)).
   */
  onFraction?: (human: string) => void
  /** Fractions of spendable to expose as preset buttons. Defaults to 25/50/75/100%. */
  fractionPresets?: number[]
  balanceLabel?: string
  testIdMax?: string
  testIdHalf?: string
  /** Per-fraction test id prefix, suffixed with the integer percent (e.g. `swap-pay-frac-25`). */
  testIdFractionPrefix?: string
}

/**
 * Shared balance row with optional 50% and Max actions ([GitLab #213](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/213)).
 */
export function AmountBalanceActions({
  balanceQuery,
  decimals,
  walletConnected,
  compact,
  showBalance = true,
  showHalf,
  spendableRaw,
  onMax,
  onHalf,
  onFraction,
  fractionPresets = [0.25, 0.5, 0.75, 1],
  balanceLabel = 'Balance:',
  testIdMax,
  testIdHalf,
  testIdFractionPrefix,
}: Props) {
  if (!walletConnected || !showBalance) return null

  const disabled =
    balanceQuery.isLoading ||
    balanceQuery.isError ||
    !balanceQuery.data ||
    balanceQuery.data === '0' ||
    spendableRaw === 0n

  const halfDisabled = balanceQuery.isLoading || balanceQuery.isError || !balanceQuery.data || balanceQuery.data === '0'

  const textSize = compact ? 'text-[10px] ' : 'text-xs '

  // Apply a fraction to the gas-adjusted spendable via exact bigint math, then
  // format to a human draft (no float drift). fraction 1 == full spendable (== Max).
  const fractionHuman = (fraction: number): string => {
    const num = BigInt(Math.round(fraction * 100))
    const scaled = (spendableRaw * num) / 100n
    return fromRawAmount(scaled.toString(), decimals)
  }

  return (
    <div
      className={textSize + 'flex flex-wrap items-center justify-between gap-2 mt-1.5 min-h-[1.25rem]'}
      style={{ color: 'var(--ink-subtle)' }}
    >
      <span className="inline-flex items-center gap-1.5 min-w-0 max-w-full">
        <span className="shrink-0">{balanceLabel}</span>
        {balanceQuery.isLoading ? (
          <span className="inline-flex items-center" aria-busy="true" aria-live="polite">
            <Spinner size="sm" className="!w-3.5 !h-3.5 opacity-90" />
            <span className="sr-only">Loading balance</span>
          </span>
        ) : balanceQuery.isError ? (
          <span className="font-mono">—</span>
        ) : (
          <span className="font-mono truncate">{formatTokenAmount(balanceQuery.data ?? '0', decimals)}</span>
        )}
      </span>
      <span className="ml-auto inline-flex items-center gap-2 shrink-0">
        {onFraction &&
          fractionPresets.map((fraction) => {
            const pct = Math.round(fraction * 100)
            return (
              <button
                key={pct}
                type="button"
                disabled={disabled}
                onClick={() => {
                  sounds.playButtonPress()
                  onFraction(fractionHuman(fraction))
                }}
                className="uppercase font-semibold tracking-wide hover:underline disabled:opacity-40 disabled:cursor-not-allowed disabled:no-underline"
                style={{ color: 'var(--cyan)' }}
                data-testid={testIdFractionPrefix ? `${testIdFractionPrefix}-${pct}` : undefined}
              >
                {pct}%
              </button>
            )
          })}
        {showHalf && onHalf && (
          <button
            type="button"
            disabled={halfDisabled}
            onClick={() => {
              sounds.playButtonPress()
              onHalf()
            }}
            className="uppercase font-semibold tracking-wide hover:underline disabled:opacity-40 disabled:cursor-not-allowed disabled:no-underline"
            style={{ color: 'var(--cyan)' }}
            data-testid={testIdHalf}
          >
            50%
          </button>
        )}
        <button
          type="button"
          disabled={disabled}
          onClick={() => {
            sounds.playButtonPress()
            onMax()
          }}
          className="uppercase font-semibold tracking-wide hover:underline disabled:opacity-40 disabled:cursor-not-allowed disabled:no-underline"
          style={{ color: 'var(--cyan)' }}
          data-testid={testIdMax}
        >
          Max
        </button>
      </span>
    </div>
  )
}
