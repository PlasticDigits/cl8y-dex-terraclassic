export type BadgeVariant = 'default' | 'success' | 'warning' | 'error' | 'neutral' | 'accent'

export interface BadgeProps {
  children: React.ReactNode
  variant?: BadgeVariant
  className?: string
}

const variantClasses: Record<BadgeVariant, string> = {
  default: 'badge-neo',
  success: 'badge-neo badge-neo-success',
  warning: 'badge-neo badge-neo-warning',
  error: 'badge-neo badge-neo-error',
  neutral: 'badge-neo',
  accent: 'badge-neo badge-neo-accent',
}

export function Badge({ children, variant = 'default', className = '' }: BadgeProps) {
  return (
    <span className={`${variantClasses[variant]} ${className}`.trim()} role="status">
      {children}
    </span>
  )
}
