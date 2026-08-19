import { formatNum } from '@/utils/formatAmount'
import { TRADER_PNL_EM_DASH } from '@/utils/traderPositionDisplay'

/** Realized P&L display with sign and color (shared by trader profile and portfolio). */
export function PnlValue({ value }: { value: string | null }) {
  if (value == null || value === '') {
    return (
      <span style={{ color: 'var(--ink-subtle)' }} className="font-bold font-heading">
        {TRADER_PNL_EM_DASH}
      </span>
    )
  }
  const n = parseFloat(value)
  const color = n > 0 ? 'var(--color-positive)' : n < 0 ? 'var(--color-negative)' : 'var(--ink-subtle)'
  const prefix = n > 0 ? '+' : ''
  return (
    <span style={{ color }} className="font-bold font-heading">
      {prefix}
      {formatNum(value, 4)}
    </span>
  )
}
