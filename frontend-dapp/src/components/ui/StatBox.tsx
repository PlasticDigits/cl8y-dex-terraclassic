import { Skeleton } from './Skeleton'

export interface StatBoxProps {
  label: string
  value: string
  loading?: boolean
  color?: string
}

export function StatBox({ label, value, loading, color }: StatBoxProps) {
  return (
    <div className="p-3 border border-white/10 rounded-sm" style={{ background: 'var(--panel-bg)' }}>
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
