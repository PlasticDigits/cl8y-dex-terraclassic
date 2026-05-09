import { humanizeUserFacingError } from '@/utils/humanizeUserFacingError'

export interface RetryErrorProps {
  message?: string
  onRetry: () => void
}

export function RetryError({ message, onRetry }: RetryErrorProps) {
  const display = humanizeUserFacingError(message ?? 'Something went wrong')
  return (
    <div className="flex flex-col items-center justify-center py-8 gap-3" aria-live="polite">
      <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
        {display}
      </p>
      <button type="button" onClick={onRetry} className="btn-primary btn-cta !text-xs !px-4 !py-1.5">
        Retry
      </button>
    </div>
  )
}
