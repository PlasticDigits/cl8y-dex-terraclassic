import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { TradesTable } from '../TradesTable'
import type { IndexerTrade } from '@/types'

const longTxHash = `AAAAAAAA${'0'.repeat(50)}BBBBBB`

const mockTrade: IndexerTrade = {
  id: 1,
  pair_address: 'terra1pair',
  block_height: 100,
  block_timestamp: '2025-01-15T12:00:00Z',
  tx_hash: longTxHash,
  sender: 'terra1trader',
  offer_asset: 'CL8Y',
  ask_asset: 'LUNC',
  offer_amount: '1000',
  return_amount: '500',
  price: '0.5',
}

const mockTrades: IndexerTrade[] = [mockTrade]

const formatTimeFn = (iso: string) => new Date(iso).toISOString()

describe('TradesTable', () => {
  it('shows "No trades" when empty', () => {
    render(<TradesTable trades={[]} formatTimeFn={formatTimeFn} />)
    expect(screen.getByText('No trades')).toBeInTheDocument()
  })

  it('renders trade rows', () => {
    render(<TradesTable trades={mockTrades} formatTimeFn={formatTimeFn} />)
    expect(screen.getByText('CL8Y → LUNC')).toBeInTheDocument()
    expect(screen.getByText('AAAAAAAA…BBBBBB')).toBeInTheDocument()
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

  it('shows hybrid badge with integrator tooltip when trade has pool/book split fields', () => {
    const hybridTrade: IndexerTrade = {
      ...mockTrade,
      id: 2,
      pool_return_amount: '100',
      book_return_amount: '400',
    }
    render(<TradesTable trades={[hybridTrade]} formatTimeFn={formatTimeFn} />)
    const badge = screen.getByTitle(/Hybrid swap:/i)
    expect(badge).toHaveTextContent('hybrid')
    expect(badge.getAttribute('title')).toMatch(/AfterSwap/)
  })
})
