import { Skeleton } from './Skeleton'

export type StatBoxVariant = 'card' | 'flat'

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
  /**
   * Default `card` keeps `card-glass` for isolated tiles.
   * Use `flat` inside a `shell-panel*` metric grid (GitLab #653) — no second radius/border/blur.
   */
  variant?: StatBoxVariant
  /** Optional second line under the value (hints, not a second chrome layer). */
  hint?: string
}

export function StatBox({
  label,
  value,
  loading,
  color,
  'data-testid': testId,
  title,
  valueAriaLabel,
  variant = 'card',
  hint,
}: StatBoxProps) {
  const accessibleValue = valueAriaLabel ?? (title ? `${title} ${value}` : undefined)
  const chrome = variant === 'flat' ? 'stat-flat' : 'card-glass !p-3'
  return (
    <div className={chrome} data-testid={testId} title={title}>
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
      {hint ? (
        <p className="text-[10px] mt-1" style={{ color: 'var(--ink-dim)' }}>
          {hint}
        </p>
      ) : null}
    </div>
  )
}
