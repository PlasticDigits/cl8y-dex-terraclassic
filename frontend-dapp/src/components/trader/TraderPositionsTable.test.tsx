import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { TraderPositionsTable } from './TraderPositionsTable'
import type { IndexerPosition } from '@/types'
import type { TraderUsdMarks } from '@/utils/traderPositionDisplay'
import { NO_COST_BASIS_LABEL } from '@/utils/traderPositionDisplay'
import * as indexerClient from '@/services/indexer/client'

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getOraclePrice: vi.fn(),
    getHubPrices: vi.fn(),
  }
})

const GEM: IndexerPosition = {
  pair_address: 'terra1ember-coral',
  asset_0_symbol: 'EMBER',
  asset_1_symbol: 'CORAL',
  asset_0_decimals: 6,
  asset_1_decimals: 6,
  net_position_quote: '1000000',
  avg_entry_price: '1',
  total_cost_base: '1000000',
  realized_pnl: '0',
  trade_count: 1,
}

const UST1_CUSTC: IndexerPosition = {
  pair_address: 'terra1ust1custc',
  asset_0_symbol: 'UST1',
  asset_1_symbol: 'cUSTC',
  asset_0_decimals: 6,
  asset_1_decimals: 6,
  net_position_quote: '38290000',
  avg_entry_price: '0.00496',
  total_cost_base: '190000',
  realized_pnl: '25000000',
  trade_count: 4,
}

const MIXED_USTR: IndexerPosition = {
  pair_address: 'terra1ust1ustr',
  asset_0_symbol: 'UST1',
  asset_1_symbol: 'USTR',
  asset_0_decimals: 6,
  asset_1_decimals: 18,
  net_position_quote: '80000000000000000000',
  avg_entry_price: '0.0000000000000125',
  total_cost_base: '1000000',
  realized_pnl: '-500000',
  trade_count: 2,
}

const HUB: TraderUsdMarks = { ust1Usd: 0.976, ustcUsd: 0.00473, ustrUsd: 0.00879 }

function renderTable(
  positions: IndexerPosition[],
  extras?: { usdMarks?: TraderUsdMarks; showTestPairDivider?: boolean }
) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  const router = createMemoryRouter(
    [
      {
        path: '/',
        element: (
          <TraderPositionsTable
            positions={positions}
            isLoading={false}
            isError={false}
            onRetry={() => {}}
            usdMarks={extras?.usdMarks}
            showTestPairDivider={extras?.showTestPairDivider}
          />
        ),
      },
    ],
    { initialEntries: ['/'] }
  )
  return render(
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  )
}

describe('TraderPositionsTable (#551)', () => {
  beforeEach(() => {
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
  it('shows human quote amounts with the quote symbol, not compact M', () => {
    renderTable([UST1_CUSTC])
    expect(screen.getByTestId('trader-position-net')).toHaveTextContent(/38\.29 cUSTC/)
    expect(screen.getByTestId('trader-position-net').textContent).not.toMatch(/M/)
    expect(screen.getByTestId('trader-position-cost')).toHaveTextContent(/UST1/)
    expect(screen.getByTestId('trader-position-pnl')).toHaveTextContent(/UST1/)
    expect(screen.getByTestId('trader-position-avg-entry')).toHaveTextContent(/UST1 \/ cUSTC/)
  })

  it('humanizes mixed-decimal avg entry instead of a T compact ratio', () => {
    renderTable([MIXED_USTR])
    expect(screen.getByTestId('trader-position-avg-entry')).toHaveTextContent(/0\.0125 UST1 \/ USTR/)
    expect(screen.getByTestId('trader-position-avg-entry').textContent).not.toMatch(/[0-9]T\b/)
    expect(screen.getByTestId('trader-position-net')).toHaveTextContent(/80(\.00)? USTR/)
    expect(screen.getByTestId('trader-position-pnl')).toHaveTextContent(/UST1/)
  })

  it('keeps a row with missing decimals as em dash (GitLab #560)', () => {
    renderTable([
      {
        pair_address: 'terra1orphan',
        asset_0_symbol: '—',
        asset_1_symbol: 'cUSTC',
        asset_0_decimals: undefined,
        asset_1_decimals: 6,
        net_position_quote: '1000000',
        avg_entry_price: '1',
        total_cost_base: '1',
        realized_pnl: '1',
        trade_count: 1,
      },
    ])
    expect(screen.getByRole('link', { name: /—\/cUSTC/ })).toBeInTheDocument()
    expect(screen.getByTestId('trader-position-cost')).toHaveTextContent('—')
    expect(screen.getByTestId('trader-position-pnl')).toHaveTextContent('—')
    expect(screen.getByTestId('trader-position-net')).toHaveTextContent(/cUSTC/)
  })

  it('inserts a Test pairs divider before the first gem row when asked (#674)', () => {
    renderTable([UST1_CUSTC, GEM], { showTestPairDivider: true })
    expect(screen.getByTestId('trader-positions-test-pairs-divider')).toHaveTextContent(/test pairs/i)
    expect(screen.getByRole('link', { name: /EMBER\/CORAL/ })).toBeInTheDocument()
  })

  it('does not insert the Test pairs divider on /trader by default', () => {
    renderTable([UST1_CUSTC, GEM])
    expect(screen.queryByTestId('trader-positions-test-pairs-divider')).not.toBeInTheDocument()
    expect(screen.getByRole('link', { name: /EMBER\/CORAL/ })).toBeInTheDocument()
  })
})

describe('TraderPositionsTable (#675 mark / unrealized)', () => {
  it('shows hub mark and unrealized beside realized for one-directional accumulation', () => {
    renderTable(
      [
        {
          ...UST1_CUSTC,
          net_position_quote: '100000000',
          total_cost_base: '400000',
          avg_entry_price: '0.004',
          realized_pnl: '0',
        },
      ],
      { usdMarks: HUB }
    )
    expect(screen.getByTestId('trader-position-pnl')).toHaveTextContent(/0(\.00)? UST1/)
    expect(screen.getByTestId('trader-position-mark')).toHaveTextContent(/\$0\.47/)
    expect(screen.getByTestId('trader-position-unrealized')).toHaveTextContent(/UST1/)
    expect(screen.getByTestId('trader-position-unrealized').textContent).toMatch(/^\+/)
  })

  it('labels no on-DEX cost basis instead of a fake P&L', () => {
    renderTable(
      [
        {
          ...UST1_CUSTC,
          net_position_quote: '100000000',
          total_cost_base: '0',
          avg_entry_price: '0',
          realized_pnl: '0',
        },
      ],
      { usdMarks: HUB }
    )
    expect(screen.getByTestId('trader-position-mark')).toHaveTextContent(/\$0\.47/)
    expect(screen.getByTestId('trader-position-unrealized')).toHaveTextContent(NO_COST_BASIS_LABEL)
  })
})
