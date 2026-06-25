export type BadgeVariant = 'default' | 'success' | 'warning' | 'error' | 'neutral' | 'accent'

export interface BadgeProps {
  children: React.ReactNode
  variant?: BadgeVariant
  className?: string
}

const variantClasses: Record<BadgeVariant, string> = {
  default: 'badge-glass',
  success: 'badge-glass badge-glass-success',
  warning: 'badge-glass badge-glass-warning',
  error: 'badge-glass badge-glass-error',
  neutral: 'badge-glass',
  accent: 'badge-glass badge-glass-accent',
}

export function Badge({ children, variant = 'default', className = '' }: BadgeProps) {
  return (
    <span className={`${variantClasses[variant]} ${className}`.trim()} role="status">
      {children}
    </span>
  )
}
