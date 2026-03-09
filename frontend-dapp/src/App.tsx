import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { Component, Suspense, lazy } from 'react'
import type { ReactNode, ErrorInfo } from 'react'
import Layout from './components/common/Layout'

const queryClient = new QueryClient()

const SwapPage = lazy(() => import('./pages/SwapPage'))
const PoolPage = lazy(() => import('./pages/PoolPage'))
const CreatePairPage = lazy(() => import('./pages/CreatePairPage'))
const TiersPage = lazy(() => import('./pages/TiersPage'))

function PageFallback() {
  return (
    <div className="flex items-center justify-center py-24">
      <span className="text-sm text-gray-400">Loading...</span>
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
        <div className="min-h-screen bg-dex-bg flex items-center justify-center p-8">
          <div className="max-w-md w-full bg-dex-card rounded-2xl border border-dex-border p-8 text-center">
            <h2 className="text-lg font-semibold text-white mb-4">Something went wrong</h2>
            <p className="text-sm text-gray-400 mb-6">
              {this.state.error?.message || 'An unexpected error occurred'}
            </p>
            <button
              onClick={() => {
                this.setState({ hasError: false, error: null })
                window.location.href = '/'
              }}
              className="px-6 py-2.5 rounded-xl font-semibold text-sm bg-dex-accent text-dex-bg hover:bg-dex-accent/80 transition-colors"
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
            </Route>
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </BrowserRouter>
      </QueryClientProvider>
    </ErrorBoundary>
  )
}
