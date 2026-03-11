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
      className={`border-2 border-[#b8ff3d] border-t-transparent rounded-full animate-spin ${sizeClasses[size]} ${className}`.trim()}
      role="status"
      aria-label="Loading"
    />
  )
}
