import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ToastProvider } from '@/contexts/ToastContext'
import { BrowserRouter, Routes, Route, Navigate, useParams } from 'react-router-dom'
import Layout from './components/common/Layout'
import { SwapAliasRedirect } from './components/common/SwapAliasRedirect'
import { ErrorBoundary } from './components/common/ErrorBoundary'
import { LazyRoute } from './components/common/LazyRoute'
import { TradePageRouteFallback } from './components/trade/TradePageRouteFallback'
import { isLcdConnectivityError } from '@/utils/lcdConnectivity'

/** Fast indexer failure for outage Playwright (GitLab #219) — avoids long retry/backoff before banners render. */
const indexerOutageE2e = import.meta.env.VITE_E2E_INDEXER_OUTAGE === '1'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: indexerOutageE2e
        ? false
        : (failureCount, error) => {
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

function TraderRouteShell() {
  const { address } = useParams<{ address?: string }>()
  return <LazyRoute loader={() => import('./pages/TraderPage')} resetKeys={[address ?? '']} />
}

export default function App() {
  return (
    <ErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <ToastProvider>
          <BrowserRouter>
            <Routes>
              <Route element={<Layout />}>
                <Route path="/" element={<LazyRoute loader={() => import('./pages/SwapPage')} />} />
                <Route path="/pool" element={<LazyRoute loader={() => import('./pages/PoolPage')} />} />
                <Route path="/create" element={<LazyRoute loader={() => import('./pages/CreatePairPage')} />} />
                <Route path="/tiers" element={<LazyRoute loader={() => import('./pages/TiersPage')} />} />
                <Route path="/charts" element={<LazyRoute loader={() => import('./pages/ChartsPage')} />} />
                <Route path="/charts/:pairAddr" element={<LazyRoute loader={() => import('./pages/ChartsPage')} />} />
                <Route path="/portfolio" element={<LazyRoute loader={() => import('./pages/PortfolioPage')} />} />
                <Route path="/my-portfolio" element={<Navigate to="/portfolio" replace />} />
                <Route path="/trader" element={<TraderRouteShell />} />
                <Route path="/trader/:address" element={<TraderRouteShell />} />
                <Route path="/limits" element={<LazyRoute loader={() => import('./pages/LimitOrdersPage')} />} />
                <Route
                  path="/trade"
                  element={
                    <LazyRoute loader={() => import('./pages/TradePage')} fallback={<TradePageRouteFallback />} />
                  }
                />
                <Route
                  path="/trade/:pairAddr"
                  element={
                    <LazyRoute loader={() => import('./pages/TradePage')} fallback={<TradePageRouteFallback />} />
                  }
                />
                <Route path="/protocol" element={<LazyRoute loader={() => import('./pages/ProtocolPage')} />} />
                <Route path="/mint" element={<LazyRoute loader={() => import('./pages/MintPage')} />} />
                <Route path="/ust1" element={<LazyRoute loader={() => import('./pages/Ust1Page')} />} />
                <Route path="/wrap" element={<LazyRoute loader={() => import('./pages/WrapPage')} />} />
                <Route path="/token/create" element={<LazyRoute loader={() => import('./pages/CreateTokenPage')} />} />
                <Route
                  path="/token/migrate"
                  element={<LazyRoute loader={() => import('./pages/MigrateTokenPage')} />}
                />
                <Route path="/tokens" element={<LazyRoute loader={() => import('./pages/MyCommunityTokensPage')} />} />
                <Route
                  path="/token/:addr/manage"
                  element={<LazyRoute loader={() => import('./pages/ManageTokenPage')} />}
                />
              </Route>
              <Route path="/swap" element={<SwapAliasRedirect />} />
              <Route path="/swap/" element={<SwapAliasRedirect />} />
              <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
          </BrowserRouter>
        </ToastProvider>
      </QueryClientProvider>
    </ErrorBoundary>
  )
}
