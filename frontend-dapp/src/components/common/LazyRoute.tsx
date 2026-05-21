import { Suspense, lazy, useMemo, useState } from 'react'
import type { ComponentType } from 'react'
import { ErrorBoundary, type ErrorBoundaryResetKey } from './ErrorBoundary'
import { Spinner } from '@/components/ui'

export function PageFallback() {
  return (
    <div className="flex items-center justify-center py-24 gap-3" style={{ color: 'var(--ink-dim)' }}>
      <Spinner />
      <span className="text-sm uppercase tracking-wide font-medium">Loading...</span>
    </div>
  )
}

type LazyModule = { default: ComponentType }

export interface LazyRouteProps {
  loader: () => Promise<LazyModule>
  resetKeys?: readonly ErrorBoundaryResetKey[]
}

/**
 * Route-scoped Suspense + ErrorBoundary. "Try Again" bumps the lazy import attempt so
 * `import()` runs again after offline chunk failures (GitLab #172).
 */
export function LazyRoute({ loader, resetKeys }: LazyRouteProps) {
  const [loadAttempt, setLoadAttempt] = useState(0)
  const Page = useMemo(() => lazy(loader), [loadAttempt])

  return (
    <ErrorBoundary isRoute resetKeys={resetKeys} onRetry={() => setLoadAttempt((n) => n + 1)}>
      <Suspense fallback={<PageFallback />}>
        <Page key={loadAttempt} />
      </Suspense>
    </ErrorBoundary>
  )
}
