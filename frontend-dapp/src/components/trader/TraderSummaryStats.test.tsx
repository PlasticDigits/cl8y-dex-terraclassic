import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { TraderSummaryStats } from './TraderSummaryStats'
import * as indexerClient from '@/services/indexer/client'
import type { IndexerPosition, IndexerTrader } from '@/types'

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getOraclePrice: vi.fn(),
  }
})

const trader: IndexerTrader = {
  address: 'terra1abcdefghijklmnopqrstuvwxyz1234567890abcd',
  total_trades: 4,
  total_volume: '10000000000000000000000000',
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

function renderSummary(pos: IndexerPosition[] = positions) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  return render(
    <QueryClientProvider client={client}>
      <TraderSummaryStats trader={trader} positions={pos} />
    </QueryClientProvider>
  )
}

describe('TraderSummaryStats (#551)', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getOraclePrice).mockImplementation(async (t = 'ustc') => ({
      ticker: t,
      price_usd: t === 'lunc' ? '0.0001' : '0.005',
      sources: [],
    }))
  })

  it('does not formatNum raw mixed volume / fees / pnl totals', async () => {
    renderSummary()
    expect(screen.getByTestId('trader-summary-volume')).toHaveTextContent('—')
    expect(screen.getByTestId('trader-summary-volume').textContent).not.toMatch(/[0-9]T\b/)
    expect(screen.getByTestId('trader-summary-fees')).toHaveTextContent('—')
    expect(screen.getByTestId('trader-summary-fees').textContent).not.toMatch(/[0-9]T\b/)
    await waitFor(() => expect(screen.getByTestId('trader-summary-realized-pnl')).toHaveTextContent(/\$/))
    expect(screen.getByTestId('trader-summary-realized-pnl').textContent).not.toMatch(/999/)
  })
})
