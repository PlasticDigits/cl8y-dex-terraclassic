import { humanizeUserFacingError } from '@/utils/humanizeUserFacingError'

export interface RetryErrorProps {
  message?: string
  onRetry: () => void
  /** When true, disables the button and shows in-flight copy ([GitLab #177](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/177)). */
  isRetrying?: boolean
  'data-testid'?: string
}

export function RetryError({ message, onRetry, isRetrying = false, 'data-testid': testId }: RetryErrorProps) {
  const display = humanizeUserFacingError(message ?? 'Something went wrong')
  return (
    <div className="flex flex-col items-center justify-center py-8 gap-3" aria-live="polite" data-testid={testId}>
      <p className="text-sm" style={{ color: 'var(--ink-dim)' }}>
        {display}
      </p>
      <button
        type="button"
        data-testid="retry-error-button"
        onClick={onRetry}
        disabled={isRetrying}
        aria-busy={isRetrying}
        className="btn-primary btn-cta !text-xs !px-4 !py-1.5 disabled:opacity-60"
      >
        {isRetrying ? 'Retrying…' : 'Retry'}
      </button>
    </div>
  )
}
