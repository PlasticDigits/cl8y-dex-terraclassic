import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter } from 'react-router-dom'
import { TraderSummaryStats } from './TraderSummaryStats'
import * as indexerClient from '@/services/indexer/client'
import * as clipboard from '@/utils/copyToClipboard'
import type { IndexerPosition, IndexerTrader } from '@/types'
import { shortenTraderAddress } from '@/utils/tokenDisplay'

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

vi.mock('@/utils/copyToClipboard', () => ({
  copyToClipboard: vi.fn().mockResolvedValue({ ok: true }),
}))

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getOraclePrice: vi.fn(),
    getHubPrices: vi.fn(),
  }
})

const ADDR = 'terra1abcdefghijklmnopqrstuvwxyz1234567890abcd'

function trader(overrides: Partial<IndexerTrader> = {}): IndexerTrader {
  return {
    address: ADDR,
    total_trades: 4,
    total_volume: '10000000000000000000000000',
    total_volume_usd: '711.2',
    volume_24h: '0',
    volume_7d: '0',
    volume_30d: '0',
    tier_id: null,
    tier_name: null,
    registered: false,
    first_trade_at: null,
    last_trade_at: null,
    total_realized_pnl: '999999999999',
    best_trade_pnl: '888888888888',
    worst_trade_pnl: '-1',
    total_fees_paid: '777777777777',
    ...overrides,
  }
}

const positions: IndexerPosition[] = [
  {
    pair_address: 'terra1a',
    asset_0_symbol: 'UST1',
    asset_1_symbol: 'cUSTC',
    asset_0_decimals: 6,
    asset_1_decimals: 6,
    net_position_quote: '0',
    avg_entry_price: '0',
    total_cost_base: '0',
    realized_pnl: '1000000',
    trade_count: 2,
  },
]

function renderSummary(row: IndexerTrader = trader(), pos: IndexerPosition[] = positions) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <TraderSummaryStats trader={row} positions={pos} />
      </MemoryRouter>
    </QueryClientProvider>
  )
}

describe('TraderSummaryStats (#551 / #560)', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getOraclePrice).mockImplementation(async (t = 'ustc') => ({
      ticker: t,
      price_usd: t === 'lunc' ? '0.0001' : '0.005',
      sources: [],
    }))
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

  it('does not formatNum raw mixed volume / fees / pnl totals', async () => {
    renderSummary()
    const volume = screen.getByTestId('trader-total-volume-usd')
    expect(volume.textContent).not.toMatch(/[0-9]T\b/)
    expect(volume.textContent).not.toMatch(/10,000,000T/)
    expect(screen.getByTestId('trader-summary-fees')).toHaveTextContent('—')
    expect(screen.getByTestId('trader-summary-fees').textContent).not.toMatch(/[0-9]T\b/)
    await waitFor(() => expect(screen.getByTestId('trader-summary-realized-pnl')).toHaveTextContent(/\$/))
    expect(screen.getByTestId('trader-summary-realized-pnl').textContent).not.toMatch(/999/)
  })

  it('uses hub UST1 mark for realized P&L, not $1 (GitLab #560)', async () => {
    renderSummary()
    await waitFor(() => expect(screen.getByTestId('trader-summary-realized-pnl')).toHaveTextContent(/\$/))
    const text = screen.getByTestId('trader-summary-realized-pnl').textContent ?? ''
    expect(text).toMatch(/\$0\.98/)
    expect(text).not.toMatch(/\$1\.00/)
    expect(indexerClient.getHubPrices).toHaveBeenCalled()
    expect(vi.mocked(indexerClient.getOraclePrice).mock.calls.map((c) => c[0])).not.toContain('ustr')
  })

  it('Best / Worst header is an em dash, not N/A (P551-5 / #560)', () => {
    renderSummary()
    expect(screen.getByTestId('trader-summary-best-trade')).toHaveTextContent('—')
    expect(screen.getByTestId('trader-summary-worst-trade')).toHaveTextContent('—')
    expect(screen.getByTestId('trader-summary-best-trade').textContent).not.toMatch(/N\/A/)
    expect(screen.getByTestId('trader-summary-worst-trade').textContent).not.toMatch(/N\/A/)
  })

  it('omits unpriced hub UST1 instead of $0', async () => {
    vi.mocked(indexerClient.getHubPrices).mockResolvedValue({
      metadata: 'DEX hub prices — not CEX',
      tickers: ['custc', 'ust1', 'ustr'],
      prices: [
        { ticker: 'custc', price_usd: '0.00473' },
        { ticker: 'ust1', price_usd: null },
        { ticker: 'ustr', price_usd: '0.00879' },
      ],
    })
    renderSummary()
    await waitFor(() => expect(screen.getByTestId('trader-summary-realized-pnl')).toHaveTextContent('—'))
    expect(screen.getByTestId('trader-summary-realized-pnl').textContent).not.toMatch(/\$0/)
  })
})

describe('TraderSummaryStats Total Volume (GitLab #553)', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getOraclePrice).mockImplementation(async (t = 'ustc') => ({
      ticker: t,
      price_usd: t === 'lunc' ? '0.0001' : '0.005',
      sources: [],
    }))
    vi.mocked(indexerClient.getHubPrices).mockResolvedValue({
      metadata: 'DEX hub prices — not CEX',
      tickers: ['custc', 'ust1', 'ustr'],
      prices: [{ ticker: 'ust1', price_usd: '0.976' }],
    })
  })

  it('formats USD compact and does not print raw USTR-scale T', () => {
    renderSummary(trader())
    const box = screen.getByTestId('trader-total-volume-usd')
    expect(box).toHaveTextContent(/total volume \(usd\)/i)
    expect(box.textContent).toMatch(/\$/)
    expect(box.textContent).not.toMatch(/10,000,000T/)
    expect(box.textContent).not.toMatch(/\dT\b/)
  })

  it('summary metric tiles are flat (GitLab #653)', () => {
    const { container } = renderSummary(trader())
    expect(screen.getByTestId('trader-total-volume-usd').className).toMatch(/stat-flat/)
    expect(screen.getByTestId('trader-total-volume-usd').className).not.toMatch(/card-glass/)
    expect(screen.getByTestId('trader-summary-realized-pnl').className).not.toMatch(/card-glass/)
    expect(container.querySelector('.shell-panel .card-glass')).toBeNull()
    expect(container.querySelector('.shell-panel-strong .card-glass')).toBeNull()
  })

  it('unpriced volume with trades is an em dash, not $0', () => {
    renderSummary(trader({ total_volume_usd: null, total_trades: 4 }))
    const box = screen.getByTestId('trader-total-volume-usd')
    expect(box).toHaveTextContent('—')
    expect(box.textContent).not.toMatch(/\$0/)
  })

  it('zero trades is $0', () => {
    renderSummary(trader({ total_trades: 0, total_volume: '0', total_volume_usd: '0' }))
    expect(screen.getByTestId('trader-total-volume-usd')).toHaveTextContent('$0')
  })
})

describe('TraderSummaryStats identity (GitLab #656)', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getOraclePrice).mockResolvedValue({
      ticker: 'ustc',
      price_usd: '0.005',
      sources: [],
    })
    vi.mocked(indexerClient.getHubPrices).mockResolvedValue({
      metadata: 'DEX hub prices — not CEX',
      tickers: ['ust1'],
      prices: [{ ticker: 'ust1', price_usd: '0.976' }],
    })
    vi.mocked(clipboard.copyToClipboard).mockClear()
  })

  it('T-ID-4/5 / A8: larger blockie + 4/6 AddressRow; copy is the full bech32', async () => {
    const user = userEvent.setup()
    renderSummary(trader(), positions)
    const identity = screen.getByTestId('trader-profile-identity')
    expect(identity.querySelector('[data-testid="trader-identity-blockie"]')).toBeTruthy()
    expect(screen.getByTestId('mock-blockies')).toHaveAttribute('data-seed', ADDR)
    expect(screen.getByTestId('trader-profile-address-row')).toHaveTextContent(shortenTraderAddress(ADDR))
    expect(screen.getByTestId('trader-profile-address-row').textContent).not.toContain(ADDR)
    expect(identity.className).not.toMatch(/card-glass/)
    await user.click(screen.getByRole('button', { name: /copy trader address/i }))
    expect(clipboard.copyToClipboard).toHaveBeenCalledWith(ADDR)
  })

  it('You badge still renders for isOwnProfile', () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    render(
      <QueryClientProvider client={client}>
        <MemoryRouter>
          <TraderSummaryStats trader={trader()} isOwnProfile />
        </MemoryRouter>
      </QueryClientProvider>
    )
    expect(screen.getByText('You')).toBeInTheDocument()
  })

  it('T-ID-6: invalid address has no blockie and no explorer /trader link', () => {
    renderSummary(trader({ address: 'javascript:alert(1)' }))
    expect(screen.queryByTestId('trader-identity-blockie')).not.toBeInTheDocument()
    expect(screen.queryByTestId('trader-profile-address-row-explorer')).not.toBeInTheDocument()
    expect(screen.queryByRole('link', { name: /trader/i })).not.toBeInTheDocument()
    document.querySelectorAll('a[href]').forEach((a) => {
      expect(a.getAttribute('href')).not.toMatch(/javascript:/i)
      expect(a.getAttribute('href')).not.toMatch(/\/trader\//)
    })
  })
})
