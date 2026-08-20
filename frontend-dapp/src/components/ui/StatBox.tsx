import { Skeleton } from './Skeleton'

export interface StatBoxProps {
  label: string
  value: string
  loading?: boolean
  color?: string
  'data-testid'?: string
  /** Progressive disclosure on the card and the label (mobile long-press). */
  title?: string
  /** Override value accessible name. Defaults to `title + value` when `title` is set. */
  valueAriaLabel?: string
}

export function StatBox({ label, value, loading, color, 'data-testid': testId, title, valueAriaLabel }: StatBoxProps) {
  const accessibleValue = valueAriaLabel ?? (title ? `${title} ${value}` : undefined)
  return (
    <div className="card-glass !p-3" data-testid={testId} title={title}>
      <p
        className="text-[10px] uppercase tracking-wider font-medium mb-1"
        style={{ color: 'var(--ink-dim)' }}
        title={title}
      >
        {label}
      </p>
      {loading ? (
        <Skeleton height="1.25rem" width="60%" />
      ) : (
        <p
          className="text-sm font-bold font-heading"
          style={{ color: color ?? 'var(--ink)' }}
          aria-label={accessibleValue}
        >
          {value}
        </p>
      )}
    </div>
  )
}
