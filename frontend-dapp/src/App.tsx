import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { BrowserRouter, Routes, Route, Navigate, useParams } from 'react-router-dom'
import { Component, Suspense, lazy } from 'react'
import type { ReactNode, ErrorInfo } from 'react'
import Layout from './components/common/Layout'
import { Spinner } from './components/ui'
import { humanizeUserFacingError } from '@/utils/humanizeUserFacingError'
import { isLcdConnectivityError } from '@/utils/lcdConnectivity'

function errorBoundaryFriendlyCopy(error: Error | null): ReactNode {
  const rawMsg = error?.message?.trim() ?? ''
  const friendly = rawMsg ? humanizeUserFacingError(rawMsg) : 'An unexpected error occurred'
  return (
    <>
      <p className="text-sm mb-6" style={{ color: 'var(--ink-dim)' }}>
        {friendly}
      </p>
      {rawMsg ? (
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
            {rawMsg}
          </pre>
        </details>
      ) : null}
    </>
  )
}

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: (failureCount, error) => {
        if (isLcdConnectivityError(error)) return failureCount < 3
        return failureCount < 2
      },
      retryDelay: (attemptIndex) => Math.min(2_000 * 2 ** attemptIndex, 12_000),
      staleTime: 10_000,
      refetchOnWindowFocus: false,
      refetchOnReconnect: true,
    },
  },
})

const SwapPage = lazy(() => import('./pages/SwapPage'))
const PoolPage = lazy(() => import('./pages/PoolPage'))
const CreatePairPage = lazy(() => import('./pages/CreatePairPage'))
const TiersPage = lazy(() => import('./pages/TiersPage'))
const ChartsPage = lazy(() => import('./pages/ChartsPage'))
const TraderPage = lazy(() => import('./pages/TraderPage'))
const ProtocolPage = lazy(() => import('./pages/ProtocolPage'))
const LimitOrdersPage = lazy(() => import('./pages/LimitOrdersPage'))
const TradePage = lazy(() => import('./pages/TradePage'))

function PageFallback() {
  return (
    <div className="flex items-center justify-center py-24 gap-3" style={{ color: 'var(--ink-dim)' }}>
      <Spinner />
      <span className="text-sm uppercase tracking-wide font-medium">Loading...</span>
    </div>
  )
}

interface ErrorBoundaryState {
  hasError: boolean
  error: Error | null
}

type ErrorBoundaryResetKey = string | number | boolean | null | undefined

function errorBoundaryResetKeysEqual(
  a: readonly ErrorBoundaryResetKey[] | undefined,
  b: readonly ErrorBoundaryResetKey[] | undefined
): boolean {
  if (!a?.length && !b?.length) return true
  if (!a || !b || a.length !== b.length) return false
  return a.every((v, i) => v === b[i])
}

interface ErrorBoundaryProps {
  children: ReactNode
  isRoute?: boolean
  /** When any entry changes, a route-level boundary clears so navigation can recover (GitLab #126). */
  resetKeys?: readonly ErrorBoundaryResetKey[]
}

class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
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

  render() {
    if (this.state.hasError) {
      if (this.props.isRoute) {
        return (
          <div className="max-w-md mx-auto py-16 text-center">
            <div className="shell-panel-strong">
              <h2
                className="text-lg font-semibold mb-4 uppercase tracking-wide font-heading"
                style={{ color: 'var(--ink)' }}
              >
                Something went wrong
              </h2>
              {errorBoundaryFriendlyCopy(this.state.error)}
              <button onClick={() => this.setState({ hasError: false, error: null })} className="btn-primary btn-cta">
                Try Again
              </button>
            </div>
          </div>
        )
      }
      return (
        <div className="min-h-screen flex items-center justify-center p-8" style={{ background: 'var(--bg-0)' }}>
          <div className="max-w-md w-full shell-panel-strong text-center">
            <h2
              className="text-lg font-semibold mb-4 uppercase tracking-wide font-heading"
              style={{ color: 'var(--ink)' }}
            >
              Something went wrong
            </h2>
            {errorBoundaryFriendlyCopy(this.state.error)}
            <button
              onClick={() => {
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

function TraderRouteShell() {
  const { address } = useParams<{ address?: string }>()
  return (
    <ErrorBoundary isRoute resetKeys={[address ?? '']}>
      <Suspense fallback={<PageFallback />}>
        <TraderPage />
      </Suspense>
    </ErrorBoundary>
  )
}

export default function App() {
  return (
    <ErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <BrowserRouter>
          <Routes>
            <Route element={<Layout />}>
              <Route
                path="/"
                element={
                  <ErrorBoundary isRoute>
                    <Suspense fallback={<PageFallback />}>
                      <SwapPage />
                    </Suspense>
                  </ErrorBoundary>
                }
              />
              <Route
                path="/pool"
                element={
                  <ErrorBoundary isRoute>
                    <Suspense fallback={<PageFallback />}>
                      <PoolPage />
                    </Suspense>
                  </ErrorBoundary>
                }
              />
              <Route
                path="/create"
                element={
                  <ErrorBoundary isRoute>
                    <Suspense fallback={<PageFallback />}>
                      <CreatePairPage />
                    </Suspense>
                  </ErrorBoundary>
                }
              />
              <Route
                path="/tiers"
                element={
                  <ErrorBoundary isRoute>
                    <Suspense fallback={<PageFallback />}>
                      <TiersPage />
                    </Suspense>
                  </ErrorBoundary>
                }
              />
              <Route
                path="/charts"
                element={
                  <ErrorBoundary isRoute>
                    <Suspense fallback={<PageFallback />}>
                      <ChartsPage />
                    </Suspense>
                  </ErrorBoundary>
                }
              />
              <Route path="/trader" element={<TraderRouteShell />} />
              <Route path="/trader/:address" element={<TraderRouteShell />} />
              <Route
                path="/limits"
                element={
                  <ErrorBoundary isRoute>
                    <Suspense fallback={<PageFallback />}>
                      <LimitOrdersPage />
                    </Suspense>
                  </ErrorBoundary>
                }
              />
              <Route
                path="/trade"
                element={
                  <ErrorBoundary isRoute>
                    <Suspense fallback={<PageFallback />}>
                      <TradePage />
                    </Suspense>
                  </ErrorBoundary>
                }
              />
              <Route
                path="/trade/:pairAddr"
                element={
                  <ErrorBoundary isRoute>
                    <Suspense fallback={<PageFallback />}>
                      <TradePage />
                    </Suspense>
                  </ErrorBoundary>
                }
              />
              <Route
                path="/protocol"
                element={
                  <ErrorBoundary isRoute>
                    <Suspense fallback={<PageFallback />}>
                      <ProtocolPage />
                    </Suspense>
                  </ErrorBoundary>
                }
              />
            </Route>
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </BrowserRouter>
      </QueryClientProvider>
    </ErrorBoundary>
  )
}
