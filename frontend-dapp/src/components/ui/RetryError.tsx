export interface RetryErrorProps {
  message?: string
  onRetry: () => void
}

export function RetryError({ message, onRetry }: RetryErrorProps) {
  return (
    <div className="flex flex-col items-center justify-center py-8 gap-3" aria-live="polite">
      <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
        {message ?? 'Something went wrong'}
      </p>
      <button onClick={onRetry} className="btn-muted !text-xs !px-4 !py-1.5">
        Retry
      </button>
    </div>
  )
}
