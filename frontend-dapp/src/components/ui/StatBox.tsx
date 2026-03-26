import { Skeleton } from './Skeleton'

export interface StatBoxProps {
  label: string
  value: string
  loading?: boolean
  color?: string
}

export function StatBox({ label, value, loading, color }: StatBoxProps) {
  return (
    <div className="card-neo !p-3">
      <p className="text-[10px] uppercase tracking-wider font-medium mb-1" style={{ color: 'var(--ink-dim)' }}>
        {label}
      </p>
      {loading ? (
        <Skeleton height="1.25rem" width="60%" />
      ) : (
        <p className="text-sm font-bold font-heading" style={{ color: color ?? 'var(--ink)' }}>
          {value}
        </p>
      )}
    </div>
  )
}
