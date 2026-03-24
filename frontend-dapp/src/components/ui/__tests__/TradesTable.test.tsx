import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { TradesTable } from '../TradesTable'
import type { IndexerTrade } from '@/types'

const mockTrades: IndexerTrade[] = [
  {
    id: 1,
    pair_address: 'terra1pair',
    block_height: 100,
    block_timestamp: '2025-01-15T12:00:00Z',
    tx_hash: 'ABCDEF1234567890',
    sender: 'terra1trader',
    offer_asset: 'CL8Y',
    ask_asset: 'LUNC',
    offer_amount: '1000',
    return_amount: '500',
    price: '0.5',
  },
]

const formatTimeFn = (iso: string) => new Date(iso).toISOString()

describe('TradesTable', () => {
  it('shows "No trades" when empty', () => {
    render(<TradesTable trades={[]} formatTimeFn={formatTimeFn} />)
    expect(screen.getByText('No trades')).toBeInTheDocument()
  })

  it('renders trade rows', () => {
    render(<TradesTable trades={mockTrades} formatTimeFn={formatTimeFn} />)
    expect(screen.getByText('CL8Y → LUNC')).toBeInTheDocument()
    expect(screen.getByText('ABCDEF12...')).toBeInTheDocument()
  })

  it('renders with aria-label', () => {
    render(<TradesTable trades={mockTrades} formatTimeFn={formatTimeFn} ariaLabel="Test trades" />)
    expect(screen.getByRole('table', { name: 'Test trades' })).toBeInTheDocument()
  })

  it('has scope=col on all header cells', () => {
    render(<TradesTable trades={mockTrades} formatTimeFn={formatTimeFn} />)
    const headers = screen.getAllByRole('columnheader')
    headers.forEach((th) => {
      expect(th).toHaveAttribute('scope', 'col')
    })
  })
})
