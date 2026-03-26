export interface SkeletonProps {
  width?: string
  height?: string
  className?: string
}

export function Skeleton({ width = '100%', height = '1rem', className = '' }: SkeletonProps) {
  return (
    <div
      className={`animate-pulse rounded-[14px] ${className}`}
      style={{ width, height, background: 'var(--ink-subtle)', opacity: 0.15 }}
      aria-hidden="true"
    />
  )
}
