import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { Suspense, lazy } from 'react'
import Layout from './components/common/Layout'

const queryClient = new QueryClient()

const SwapPage = lazy(() => import('./pages/SwapPage'))
const PoolPage = lazy(() => import('./pages/PoolPage'))
const CreatePairPage = lazy(() => import('./pages/CreatePairPage'))

function PageFallback() {
  return (
    <div className="flex items-center justify-center py-24">
      <span className="text-sm text-gray-400">Loading...</span>
    </div>
  )
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <Routes>
          <Route element={<Layout />}>
            <Route path="/" element={<Suspense fallback={<PageFallback />}><SwapPage /></Suspense>} />
            <Route path="/pool" element={<Suspense fallback={<PageFallback />}><PoolPage /></Suspense>} />
            <Route path="/create" element={<Suspense fallback={<PageFallback />}><CreatePairPage /></Suspense>} />
          </Route>
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  )
}
