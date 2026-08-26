import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import TraderPage from './TraderPage'
import * as indexerClient from '@/services/indexer/client'
import type { IndexerTrader } from '@/types'

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
    getOraclePrice: vi.fn(),
    getHubPrices: vi.fn(),
    getLeaderboard: vi.fn(),
  }
})

const TRADER_ADDR = 'terra1trader000000000000000000000000000000aa'
const PEER_ADDR = 'terra1abcdefghijklmnopqrstuvwxyz1234567890abcd'

function mockTrader(overrides: Partial<IndexerTrader> = {}): IndexerTrader {
  return {
    address: TRADER_ADDR,
    total_trades: 4,
    total_volume: '1000',
    total_volume_usd: '711.2',
    volume_24h: '0',
    volume_7d: '0',
    volume_30d: '0',
    tier_id: null,
    tier_name: null,
    registered: false,
    first_trade_at: null,
    last_trade_at: null,
    total_realized_pnl: '0',
    best_trade_pnl: null,
    worst_trade_pnl: null,
    total_fees_paid: '0',
    ...overrides,
  }
}

function renderTraderAt(path: string) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  })
  const router = createMemoryRouter(
    [
      { path: '/trader', element: <TraderPage /> },
      { path: '/trader/:address', element: <TraderPage /> },
    ],
    { initialEntries: [path] }
  )
  return render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  )
}

describe('TraderPage (component)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(indexerClient.getTraderTrades).mockResolvedValue([])
    vi.mocked(indexerClient.getTraderPositions).mockResolvedValue([])
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([])
    vi.mocked(indexerClient.getTrader).mockReset()
    vi.mocked(indexerClient.getOraclePrice).mockResolvedValue({
      ticker: 'ustc',
      price_usd: '0.005',
      sources: [],
    })
    vi.mocked(indexerClient.getHubPrices).mockResolvedValue({
      metadata: 'DEX hub prices — not CEX',
      tickers: ['custc', 'ust1', 'ustr'],
      prices: [
        { ticker: 'custc', price_usd: '0.00473' },
        { ticker: 'ust1', price_usd: '0.976' },
        { ticker: 'ustr', price_usd: '0.00879' },
      ],
    })
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

  it('/trader empty lookup shows Leaderboard after the empty prompt (TL-1 / TL-2)', async () => {
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([mockTrader({ address: PEER_ADDR })])
    renderTraderAt('/trader')
    const prompt = await screen.findByText(/search for a trader wallet/i)
    const board = await screen.findByTestId('trader-leaderboard')
    expect(prompt.compareDocumentPosition(board) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(screen.getByRole('heading', { name: /^leaderboard$/i })).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: /trade history/i })).not.toBeInTheDocument()
  })

  it('empty lookup still shows board RetryError when getLeaderboard fails', async () => {
    vi.mocked(indexerClient.getLeaderboard).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderTraderAt('/trader')
    expect(await screen.findByText(/search for a trader wallet/i)).toBeInTheDocument()
    expect(await screen.findByText(/failed to load leaderboard/i)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /search/i })).toBeInTheDocument()
  })

  it('profile found: board is after Trade History (TL-1)', async () => {
    vi.mocked(indexerClient.getTrader).mockResolvedValue(mockTrader())
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([mockTrader({ address: PEER_ADDR })])
    renderTraderAt(`/trader/${TRADER_ADDR}`)
    const history = await screen.findByRole('heading', { name: /trade history/i })
    const board = await screen.findByTestId('trader-leaderboard')
    expect(history.compareDocumentPosition(board) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('404 profile still mounts the board; no outage banner (TL-2 / #177)', async () => {
    vi.mocked(indexerClient.getTrader).mockRejectedValue(new Error('Indexer API error: 404 Not Found'))
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([mockTrader({ address: PEER_ADDR })])
    renderTraderAt(`/trader/${TRADER_ADDR}`)
    await waitFor(() => expect(screen.getByText(/trader not found/i)).toBeInTheDocument())
    expect(screen.queryByTestId('trader-market-data-outage-banner')).not.toBeInTheDocument()
    expect(await screen.findByTestId('trader-leaderboard')).toBeInTheDocument()
    expect(await screen.findByRole('table', { name: /trader leaderboard/i })).toBeInTheDocument()
  })

  it('profile outage still mounts the board (TL-2 / #215)', async () => {
    vi.mocked(indexerClient.getTrader).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([mockTrader({ address: PEER_ADDR })])
    renderTraderAt(`/trader/${TRADER_ADDR}`)
    expect(await screen.findByTestId('trader-market-data-outage-banner')).toBeInTheDocument()
    expect(await screen.findByTestId('trader-leaderboard')).toBeInTheDocument()
    expect(await screen.findByRole('table', { name: /trader leaderboard/i })).toBeInTheDocument()
  })

  it('both fail: banner + board RetryError; no env/host leak', async () => {
    vi.mocked(indexerClient.getTrader).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getLeaderboard).mockRejectedValue(new Error('connect ECONNREFUSED 127.0.0.1:3001'))
    renderTraderAt(`/trader/${TRADER_ADDR}`)
    const banner = await screen.findByTestId('trader-market-data-outage-banner')
    expect(await screen.findByText(/failed to load leaderboard/i)).toBeInTheDocument()
    expect(`${banner.textContent}\n${screen.getByTestId('trader-leaderboard').textContent}`).not.toMatch(
      /VITE_INDEXER_URL|127\.0\.0\.1/i
    )
  })

  it('highlights the current profile row when it is in the top 20', async () => {
    vi.mocked(indexerClient.getTrader).mockResolvedValue(mockTrader())
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([
      mockTrader({ address: PEER_ADDR }),
      mockTrader({ address: TRADER_ADDR }),
    ])
    renderTraderAt(`/trader/${TRADER_ADDR}`)
    const table = await screen.findByRole('table', { name: /trader leaderboard/i })
    const current = table.querySelector('[aria-current="page"]')
    expect(current).toBeTruthy()
    expect(current?.querySelector('a')?.getAttribute('href')).toBe(`/trader/${TRADER_ADDR}`)
  })

  it('invalid search does not navigate; board stays mounted', async () => {
    const user = userEvent.setup()
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([mockTrader({ address: PEER_ADDR })])
    renderTraderAt('/trader')
    await screen.findByRole('table', { name: /trader leaderboard/i })
    await user.type(screen.getByLabelText(/trader wallet address/i), 'not-a-wallet')
    await user.click(screen.getByRole('button', { name: /search/i }))
    expect(screen.getByText(/search for a trader wallet/i)).toBeInTheDocument()
    expect(screen.getByTestId('trader-leaderboard')).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: /trade history/i })).not.toBeInTheDocument()
  })
})
