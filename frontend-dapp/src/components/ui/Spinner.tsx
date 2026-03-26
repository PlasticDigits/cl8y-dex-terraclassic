export interface SpinnerProps {
  className?: string
  size?: 'sm' | 'md' | 'lg'
}

const sizeClasses = {
  sm: 'w-4 h-4',
  md: 'w-6 h-6',
  lg: 'w-8 h-8',
}

export function Spinner({ className = '', size = 'md' }: SpinnerProps) {
  return (
    <div
      className={`animate-spin rounded-full border-2 border-t-transparent ${sizeClasses[size]} ${className}`.trim()}
      style={{ borderColor: 'var(--mint)', borderTopColor: 'transparent' }}
      role="status"
      aria-label="Loading"
    />
  )
}
