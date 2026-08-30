import { Suspense, lazy, useMemo, useState, type ReactNode } from 'react'
import type { ComponentType } from 'react'
import { ErrorBoundary, type ErrorBoundaryResetKey } from './ErrorBoundary'
import { RouteContentReadyMarker } from './RouteContentReadyMarker'
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
  /** Route-specific Suspense fallback; defaults to {@link PageFallback}. */
  fallback?: ReactNode
}

/**
 * Route-scoped Suspense + ErrorBoundary. "Try Again" bumps the lazy import attempt so
 * `import()` runs again after offline chunk failures (GitLab #172). Online stale-hash
 * 404s one-shot reload the document before painting Page unavailable (GitLab #706).
 */
export function LazyRoute({ loader, resetKeys, fallback }: LazyRouteProps) {
  const [loadAttempt, setLoadAttempt] = useState(0)
  const Page = useMemo(() => lazy(loader), [loadAttempt])

  return (
    <ErrorBoundary isRoute resetKeys={resetKeys} onRetry={() => setLoadAttempt((n) => n + 1)}>
      <Suspense fallback={fallback ?? <PageFallback />}>
        <RouteContentReadyMarker>
          <Page key={loadAttempt} />
        </RouteContentReadyMarker>
      </Suspense>
    </ErrorBoundary>
  )
}
