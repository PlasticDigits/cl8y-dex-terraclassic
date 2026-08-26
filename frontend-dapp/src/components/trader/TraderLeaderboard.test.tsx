import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { TraderLeaderboard } from './TraderLeaderboard'
import { filterLeaderboardRows } from './traderLeaderboard'
import * as indexerClient from '@/services/indexer/client'
import type { IndexerTrader } from '@/types'

vi.mock('react-blockies', () => ({
  __esModule: true,
  default: function MockBlockies({ seed }: { seed: string }) {
    return <span data-testid="mock-blockies" data-seed={seed} />
  },
}))

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
    getLeaderboard: vi.fn(),
  }
})

const ADDR = 'terra1abcdefghijklmnopqrstuvwxyz1234567890abcd'
const ADDR_B = 'terra1bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'

function trader(overrides: Partial<IndexerTrader> = {}): IndexerTrader {
  return {
    address: ADDR,
    total_trades: 4,
    total_volume: '10000000000000000000',
    total_volume_usd: '711.2',
    volume_24h: '0',
    volume_7d: '0',
    volume_30d: '0',
    tier_id: null,
    tier_name: null,
    registered: false,
    first_trade_at: null,
    last_trade_at: null,
    total_realized_pnl: '12.5',
    best_trade_pnl: '3.2',
    worst_trade_pnl: '-1.1',
    total_fees_paid: '0',
    ...overrides,
  }
}

function renderBoard(highlightAddress?: string) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  })
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={['/trader']}>
        <Routes>
          <Route path="/trader" element={<TraderLeaderboard highlightAddress={highlightAddress} />} />
          <Route path="/trader/:address" element={<div data-testid="trader-profile-stub" />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>
  )
}

describe('filterLeaderboardRows', () => {
  it('omits non-bech32, javascript:, and protocol-relative addresses', () => {
    const rows = filterLeaderboardRows([
      trader(),
      { ...trader(), address: '<script>alert(1)</script>' },
      { ...trader(), address: 'javascript:alert(1)' },
      { ...trader(), address: '//evil.example' },
      { ...trader(), address: 'https://evil.example' },
      null,
      { constructor: { prototype: {} } },
      { address: ADDR_B, total_trades: 1, total_volume: '0', total_volume_usd: '1' },
    ])
    expect(rows.map((r) => r.address)).toEqual([ADDR, ADDR_B])
  })

  it('returns [] for non-arrays', () => {
    expect(filterLeaderboardRows(null)).toEqual([])
    expect(filterLeaderboardRows(undefined)).toEqual([])
  })
})

describe('TraderLeaderboard', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([])
  })

  it('requests total_volume_usd with limit 20 on first paint (TL-3 / T553-5)', async () => {
    renderBoard()
    await waitFor(() => expect(indexerClient.getLeaderboard).toHaveBeenCalledWith('total_volume_usd', 20))
    expect(screen.getByRole('tab', { name: /volume \(usd\)/i })).toHaveAttribute('aria-selected', 'true')
  })

  it('Volume column is USD compact; raw USTR-scale total_volume is not shown as T (T553-1)', async () => {
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([trader()])
    renderBoard()
    const cell = await screen.findByTestId('charts-leaderboard-volume')
    expect(cell.textContent).toMatch(/\$/)
    expect(cell.textContent).not.toMatch(/10,000,000T/)
    expect(cell.textContent).not.toMatch(/\dT\b/)
    expect(screen.getByTestId('trader-leaderboard').innerHTML).not.toMatch(/10000000000000000000/)
  })

  it('unpriced leaderboard volume is an em dash, not $0', async () => {
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([trader({ total_volume_usd: null, total_trades: 2 })])
    renderBoard()
    const cell = await screen.findByTestId('charts-leaderboard-volume')
    expect(cell).toHaveTextContent('—')
    expect(cell.textContent).not.toMatch(/\$0/)
  })

  it('idle row (total_trades === 0) shows $0', async () => {
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([trader({ total_volume_usd: '0', total_trades: 0 })])
    renderBoard()
    const cell = await screen.findByTestId('charts-leaderboard-volume')
    expect(cell).toHaveTextContent('$0')
  })

  it('empty indexer shows No traders yet', async () => {
    renderBoard()
    expect(await screen.findByText(/no traders yet/i)).toBeInTheDocument()
  })

  it('load failure shows RetryError without env or host leak', async () => {
    vi.mocked(indexerClient.getLeaderboard).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderBoard()
    expect(await screen.findByText(/failed to load leaderboard/i)).toBeInTheDocument()
    expect(screen.getByTestId('retry-error-button')).toBeInTheDocument()
    expect(screen.getByTestId('trader-leaderboard').textContent).not.toMatch(/VITE_INDEXER_URL|127\.0\.0\.1/i)
  })

  it('P&L tabs request matching sort and render PnlValue fields', async () => {
    const user = userEvent.setup()
    vi.mocked(indexerClient.getLeaderboard).mockImplementation(async (sort) => {
      if (sort === 'best_trade_pnl') return [trader({ best_trade_pnl: '3.2' })]
      if (sort === 'total_realized_pnl') return [trader({ total_realized_pnl: '12.5' })]
      if (sort === 'worst_trade_pnl') return [trader({ worst_trade_pnl: '-1.1' })]
      return [trader()]
    })
    renderBoard()
    await screen.findByTestId('charts-leaderboard-volume')

    await user.click(screen.getByRole('tab', { name: /best trade/i }))
    await waitFor(() => expect(indexerClient.getLeaderboard).toHaveBeenCalledWith('best_trade_pnl', 20))
    expect(screen.getByRole('table', { name: /trader leaderboard/i })).toHaveTextContent('+3.200')

    await user.click(screen.getByRole('tab', { name: /most profit/i }))
    await waitFor(() => expect(indexerClient.getLeaderboard).toHaveBeenCalledWith('total_realized_pnl', 20))
    expect(screen.getByRole('table', { name: /trader leaderboard/i })).toHaveTextContent('+12.50')

    await user.click(screen.getByRole('tab', { name: /most loss/i }))
    await waitFor(() => expect(indexerClient.getLeaderboard).toHaveBeenCalledWith('worst_trade_pnl', 20))
    expect(screen.getByRole('table', { name: /trader leaderboard/i })).toHaveTextContent('-1.100')
  })

  it('row link is path-absolute /trader/{addr} with encodeURIComponent', async () => {
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([trader()])
    renderBoard()
    const table = await screen.findByRole('table', { name: /trader leaderboard/i })
    const link = within(table).getByRole('link')
    expect(link).toHaveAttribute('href', `/trader/${ADDR}`)
    expect(link.getAttribute('href')).not.toMatch(/javascript:/)
    expect(link).toHaveAttribute('data-testid', 'charts-leaderboard-trader')
    expect(link.textContent).not.toBe(ADDR)
    expect(link.textContent).toMatch(/terr…/)
  })

  it('highlights the current address row when it is in the top 20 (TL-9)', async () => {
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([trader({ address: ADDR_B }), trader({ address: ADDR })])
    renderBoard(ADDR)
    const table = await screen.findByRole('table', { name: /trader leaderboard/i })
    const rows = within(table).getAllByRole('row')
    const dataRows = rows.slice(1)
    expect(dataRows[0]).not.toHaveAttribute('aria-current')
    expect(dataRows[1]).toHaveAttribute('aria-current', 'page')
  })

  it('does not invent a rank when highlightAddress is absent from the page', async () => {
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([trader({ address: ADDR_B })])
    renderBoard(ADDR)
    const table = await screen.findByRole('table', { name: /trader leaderboard/i })
    expect(within(table).queryByText(/you are rank/i)).not.toBeInTheDocument()
    expect(within(table).getAllByRole('row')[1]).not.toHaveAttribute('aria-current')
  })

  it('omits XSS / open-redirect addresses from the table', async () => {
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([
      trader({ address: '<script>alert(1)</script>' }),
      trader({ address: 'javascript:alert(1)' }),
      trader({ address: ADDR }),
    ] as IndexerTrader[])
    renderBoard()
    const table = await screen.findByRole('table', { name: /trader leaderboard/i })
    expect(table.innerHTML).not.toMatch(/<script>/)
    expect(table.innerHTML).not.toMatch(/javascript:/)
    expect(within(table).getAllByRole('link')).toHaveLength(1)
    expect(within(table).getByRole('link')).toHaveAttribute('href', `/trader/${ADDR}`)
  })

  it('does not re-sort server order (rank is list index)', async () => {
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([
      trader({ address: ADDR_B, total_volume_usd: '1' }),
      trader({ address: ADDR, total_volume_usd: '999' }),
    ])
    renderBoard()
    const table = await screen.findByRole('table', { name: /trader leaderboard/i })
    const cells = within(table)
      .getAllByRole('row')
      .slice(1)
      .map((row) => within(row).getAllByRole('cell')[0].textContent)
    expect(cells).toEqual(['1', '2'])
    const links = within(table).getAllByRole('link')
    expect(links[0]).toHaveAttribute('href', `/trader/${ADDR_B}`)
    expect(links[1]).toHaveAttribute('href', `/trader/${ADDR}`)
  })

  it('does not expose a limit or free-text sort control', async () => {
    renderBoard()
    await screen.findByRole('heading', { name: /^leaderboard$/i })
    expect(screen.queryByRole('spinbutton')).not.toBeInTheDocument()
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
    expect(screen.getByTestId('trader-leaderboard').innerHTML).not.toMatch(/dangerouslySetInnerHTML/)
  })

  it('one chrome layer: shell-panel-strong without nested shell-panel or card-glass (TL-5)', async () => {
    vi.mocked(indexerClient.getLeaderboard).mockResolvedValue([trader()])
    renderBoard()
    const root = await screen.findByTestId('trader-leaderboard')
    expect(root.className).toMatch(/shell-panel-strong/)
    expect(root.querySelector('[class*="shell-panel"]')).toBeNull()
    expect(root.innerHTML).not.toMatch(/card-glass/)
  })
})
