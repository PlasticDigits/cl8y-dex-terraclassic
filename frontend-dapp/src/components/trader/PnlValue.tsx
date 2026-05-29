import { formatNum } from '@/utils/formatAmount'

/** Realized P&amp;L display with sign and color (shared by trader profile and portfolio). */
export function PnlValue({ value }: { value: string }) {
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
