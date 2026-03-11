export interface CardProps {
  children: React.ReactNode
  className?: string
  strong?: boolean
}

export function Card({ children, className = '', strong = false }: CardProps) {
  const base = strong ? 'shell-panel-strong' : 'card-neo'
  return <div className={`${base} ${className}`.trim()}>{children}</div>
}
