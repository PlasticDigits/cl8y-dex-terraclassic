import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import TraderPage from './TraderPage'
import * as indexerClient from '@/services/indexer/client'

vi.mock('@/lib/sounds', () => ({
  sounds: {
    playButtonPress: vi.fn(),
    playHover: vi.fn(),
    playSuccess: vi.fn(),
    playError: vi.fn(),
  },
}))

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getTrader: vi.fn(),
    getTraderTrades: vi.fn(),
    getTraderPositions: vi.fn(),
  }
})

const TRADER_ADDR = 'terra1trader000000000000000000000000000000'

function renderTraderAt(path: string) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  })
  const router = createMemoryRouter([{ path: '/trader/:address', element: <TraderPage /> }], {
    initialEntries: [path],
  })
  return render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  )
}

describe('TraderPage (component)', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getTraderTrades).mockResolvedValue([])
    vi.mocked(indexerClient.getTraderPositions).mockResolvedValue([])
  })

  it('shows retail market-data banner on indexer transport failure (GitLab #215)', async () => {
    vi.mocked(indexerClient.getTrader).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderTraderAt(`/trader/${TRADER_ADDR}`)
    const banner = await screen.findByTestId('trader-market-data-outage-banner')
    expect(banner).toHaveTextContent(/market data service unavailable/i)
    expect(banner.textContent).not.toMatch(/VITE_INDEXER_URL|127\.0\.0\.1/i)
  })

  it('shows not-found RetryError on 404, not outage banner (GitLab #177)', async () => {
    vi.mocked(indexerClient.getTrader).mockRejectedValue(new Error('Indexer API error: 404 Not Found'))
    renderTraderAt(`/trader/${TRADER_ADDR}`)
    await waitFor(() => expect(screen.getByText(/trader not found/i)).toBeInTheDocument())
    expect(screen.queryByTestId('trader-market-data-outage-banner')).not.toBeInTheDocument()
  })
})
