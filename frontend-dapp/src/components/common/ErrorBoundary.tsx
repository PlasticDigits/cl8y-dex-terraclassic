import { Component } from 'react'
import type { ErrorInfo, ReactNode } from 'react'
import { humanizeUserFacingError } from '@/utils/humanizeUserFacingError'
import { CHUNK_LOAD_ROUTE_HEADLINE, isChunkLoadError, sanitizeChunkLoadTechnicalDetail } from '@/utils/chunkLoadError'

function errorBoundaryFriendlyCopy(error: Error | null): ReactNode {
  const rawMsg = error?.message?.trim() ?? ''
  const friendly = rawMsg ? humanizeUserFacingError(rawMsg) : 'An unexpected error occurred'
  const technical = rawMsg ? sanitizeChunkLoadTechnicalDetail(rawMsg) : ''
  return (
    <>
      <p className="text-sm mb-6" style={{ color: 'var(--ink-dim)' }}>
        {friendly}
      </p>
      {technical ? (
        <details className="mb-6 text-left text-xs max-w-full" style={{ color: 'var(--ink-subtle)' }}>
          <summary
            className="cursor-pointer select-none uppercase tracking-wide font-medium mb-2 list-none [&::-webkit-details-marker]:hidden"
            style={{ color: 'var(--ink-dim)' }}
          >
            Technical details
          </summary>
          <pre
            className="whitespace-pre-wrap break-words font-mono text-[11px] p-2 rounded max-h-40 overflow-auto shell-panel"
            style={{ color: 'var(--ink)' }}
          >
            {technical}
          </pre>
        </details>
      ) : null}
    </>
  )
}

interface ErrorBoundaryState {
  hasError: boolean
  error: Error | null
}

export type ErrorBoundaryResetKey = string | number | boolean | null | undefined

function errorBoundaryResetKeysEqual(
  a: readonly ErrorBoundaryResetKey[] | undefined,
  b: readonly ErrorBoundaryResetKey[] | undefined
): boolean {
  if (!a?.length && !b?.length) return true
  if (!a || !b || a.length !== b.length) return false
  return a.every((v, i) => v === b[i])
}

export interface ErrorBoundaryProps {
  children: ReactNode
  isRoute?: boolean
  /** When any entry changes, a route-level boundary clears so navigation can recover (GitLab #126). */
  resetKeys?: readonly ErrorBoundaryResetKey[]
  /** Re-run lazy `import()` after chunk failures (GitLab #172). */
  onRetry?: () => void
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('[ErrorBoundary] Unhandled error:', error, errorInfo)
  }

  componentDidUpdate(prevProps: ErrorBoundaryProps) {
    const { resetKeys } = this.props
    if (!this.state.hasError || !resetKeys?.length) return
    if (!errorBoundaryResetKeysEqual(prevProps.resetKeys, resetKeys)) {
      this.setState({ hasError: false, error: null })
    }
  }

  private handleRetry = () => {
    this.props.onRetry?.()
    this.setState({ hasError: false, error: null })
  }

  render() {
    if (this.state.hasError) {
      const chunkLoad = isChunkLoadError(this.state.error)
      const headline = this.props.isRoute && chunkLoad ? CHUNK_LOAD_ROUTE_HEADLINE : 'Something went wrong'

      if (this.props.isRoute) {
        return (
          <div className="max-w-md mx-auto py-16 text-center" data-testid="route-error-boundary">
            <div className="shell-panel-strong">
              <h2
                className="text-lg font-semibold mb-4 uppercase tracking-wide font-heading"
                style={{ color: 'var(--ink)' }}
              >
                {headline}
              </h2>
              {errorBoundaryFriendlyCopy(this.state.error)}
              <button type="button" onClick={this.handleRetry} className="btn-primary btn-cta">
                Try Again
              </button>
            </div>
          </div>
        )
      }
      return (
        <div className="min-h-screen flex items-center justify-center p-8" style={{ background: 'var(--bg-0)' }}>
          <div className="max-w-md w-full shell-panel-strong text-center" data-testid="app-error-boundary">
            <h2
              className="text-lg font-semibold mb-4 uppercase tracking-wide font-heading"
              style={{ color: 'var(--ink)' }}
            >
              {headline}
            </h2>
            {errorBoundaryFriendlyCopy(this.state.error)}
            <button
              type="button"
              onClick={() => {
                if (chunkLoad) {
                  window.location.reload()
                  return
                }
                this.setState({ hasError: false, error: null })
                window.location.href = '/'
              }}
              className="btn-primary btn-cta"
            >
              Reload App
            </button>
          </div>
        </div>
      )
    }
    return this.props.children
  }
}
