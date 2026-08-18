import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { TraderPositionsTable } from './TraderPositionsTable'
import type { IndexerPosition } from '@/types'

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

function renderTable(positions: IndexerPosition[]) {
  const router = createMemoryRouter(
    [
      {
        path: '/',
        element: <TraderPositionsTable positions={positions} isLoading={false} isError={false} onRetry={() => {}} />,
      },
    ],
    { initialEntries: ['/'] }
  )
  return render(<RouterProvider router={router} />)
}

describe('TraderPositionsTable (#551)', () => {
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
})
