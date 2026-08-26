import { Skeleton } from './Skeleton'
import { protocolPctToneFromDisplay } from '@/utils/formatProtocolStats'
import { composeStatAriaLabel } from '@/utils/trailingWindowCopy'

export interface StatDelta {
  value: string
  label: string
  testId?: string
  title?: string
}

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
  /** Default `'card'` keeps Swap/Trade/Charts callers on `card-glass`. */
  variant?: 'card' | 'flat'
  /** Single Δ% (volume / fees). Prefer `deltas` for liquidity 24h+30d. */
  delta?: string
  deltaLabel?: string
  deltaTestId?: string
  deltaTitle?: string
  deltas?: StatDelta[]
}

function resolvedDeltas(props: StatBoxProps): StatDelta[] {
  if (props.deltas?.length) return props.deltas
  if (props.delta != null) {
    return [
      {
        value: props.delta,
        label: props.deltaLabel ?? '',
        testId: props.deltaTestId,
        title: props.deltaTitle,
      },
    ]
  }
  return []
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
  delta,
  deltaLabel,
  deltaTestId,
  deltaTitle,
  deltas,
}: StatBoxProps) {
  const items = resolvedDeltas({ delta, deltaLabel, deltaTestId, deltaTitle, deltas })
  const deltaPhrase = items
    .map((d) => [d.value, d.label].filter(Boolean).join(' '))
    .filter(Boolean)
    .join(', ')
  const accessibleValue =
    valueAriaLabel ?? (title ? composeStatAriaLabel(title, deltaPhrase ? `${value} ${deltaPhrase}` : value) : undefined)
  const shell = variant === 'flat' ? 'stat-box-flat py-1 min-w-0' : 'card-glass !p-3'

  return (
    <div className={shell} data-testid={testId} title={title}>
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
        <div className="flex items-baseline justify-between gap-2 min-w-0 flex-wrap">
          <p
            className="text-sm font-bold font-heading min-w-0"
            style={{ color: color ?? 'var(--ink)' }}
            aria-label={accessibleValue}
          >
            {value}
          </p>
          {items.length > 0 && (
            <div className="flex items-baseline gap-2 shrink-0">
              {items.map((d) => (
                <span
                  key={`${d.testId ?? d.label}-${d.value}`}
                  data-testid={d.testId}
                  className="text-xs font-semibold tabular-nums whitespace-nowrap"
                  style={{ color: protocolPctToneFromDisplay(d.value) }}
                  title={d.title}
                  aria-label={d.title ? composeStatAriaLabel(d.title, `${d.value} ${d.label}`.trim()) : undefined}
                >
                  {d.value}
                  {d.label ? (
                    <span className="ml-0.5 font-medium" style={{ color: 'var(--ink-dim)' }}>
                      {d.label}
                    </span>
                  ) : null}
                </span>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
