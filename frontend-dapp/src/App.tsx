import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { Component, Suspense, lazy } from 'react'
import type { ReactNode, ErrorInfo } from 'react'
import Layout from './components/common/Layout'
import { Spinner } from './components/ui'

const queryClient = new QueryClient()

const SwapPage = lazy(() => import('./pages/SwapPage'))
const PoolPage = lazy(() => import('./pages/PoolPage'))
const CreatePairPage = lazy(() => import('./pages/CreatePairPage'))
const TiersPage = lazy(() => import('./pages/TiersPage'))
const ChartsPage = lazy(() => import('./pages/ChartsPage'))
const TraderPage = lazy(() => import('./pages/TraderPage'))

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

class ErrorBoundary extends Component<{ children: ReactNode }, ErrorBoundaryState> {
  constructor(props: { children: ReactNode }) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('[ErrorBoundary] Unhandled error:', error, errorInfo)
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-screen flex items-center justify-center p-8" style={{ background: 'var(--bg-0)' }}>
          <div className="max-w-md w-full shell-panel-strong text-center">
            <h2
              className="text-lg font-semibold mb-4 uppercase tracking-wide"
              style={{ color: 'var(--ink)', fontFamily: "'Chakra Petch', sans-serif" }}
            >
              Something went wrong
            </h2>
            <p className="text-sm mb-6" style={{ color: 'var(--ink-dim)' }}>
              {this.state.error?.message || 'An unexpected error occurred'}
            </p>
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

export default function App() {
  return (
    <ErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <BrowserRouter>
          <Routes>
            <Route element={<Layout />}>
              <Route path="/" element={<Suspense fallback={<PageFallback />}><SwapPage /></Suspense>} />
              <Route path="/pool" element={<Suspense fallback={<PageFallback />}><PoolPage /></Suspense>} />
              <Route path="/create" element={<Suspense fallback={<PageFallback />}><CreatePairPage /></Suspense>} />
              <Route path="/tiers" element={<Suspense fallback={<PageFallback />}><TiersPage /></Suspense>} />
              <Route path="/charts" element={<Suspense fallback={<PageFallback />}><ChartsPage /></Suspense>} />
              <Route path="/trader/:address" element={<Suspense fallback={<PageFallback />}><TraderPage /></Suspense>} />
            </Route>
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </BrowserRouter>
      </QueryClientProvider>
    </ErrorBoundary>
  )
}
