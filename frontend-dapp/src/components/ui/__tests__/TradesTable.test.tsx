import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { TradesTable } from '../TradesTable'
import type { IndexerPair, IndexerTrade } from '@/types'

const longTxHash = `AAAAAAAA${'0'.repeat(50)}BBBBBB`

const mockPair: IndexerPair = {
  pair_address: 'terra1pair',
  asset_0: { symbol: 'UST1', contract_addr: 'terra1ust1', denom: null, decimals: 6 },
  asset_1: { symbol: 'cUSTC', contract_addr: 'terra1custc', denom: null, decimals: 6 },
  lp_token: null,
  fee_bps: 30,
  is_active: true,
}

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

  it('renders column headers for tape (GitLab #149)', () => {
    render(<TradesTable trades={mockTrades} formatTimeFn={formatTimeFn} />)
    expect(screen.getByRole('columnheader', { name: /pair/i })).toBeInTheDocument()
    expect(screen.getByRole('columnheader', { name: /^amount in$/i })).toBeInTheDocument()
    expect(screen.getByRole('columnheader', { name: /^amount out$/i })).toBeInTheDocument()
  })

  it('renders trade rows', () => {
    render(<TradesTable trades={mockTrades} formatTimeFn={formatTimeFn} />)
    expect(screen.getByText('CL8Y → LUNC')).toBeInTheDocument()
    expect(screen.getByText('AAAAAAAA…BBBBBB')).toBeInTheDocument()
  })

  it('does not compact-format a mixed-decimal raw price as T (GitLab #522)', () => {
    render(<TradesTable trades={[{ ...mockTrade, id: 3, price: '79718100000000' }]} formatTimeFn={formatTimeFn} />)
    expect(screen.queryByText(/T$/)).not.toBeInTheDocument()
    expect(screen.getByText(/79,718,100,000,000/)).toBeInTheDocument()
  })

  it('humanizes UST1/cUSTC raw amounts and keeps human price (GitLab #557)', () => {
    const row: IndexerTrade = {
      ...mockTrade,
      offer_asset: 'UST1',
      ask_asset: 'cUSTC',
      offer_amount: '1000000',
      return_amount: '206000000',
      offer_decimals: 6,
      ask_decimals: 6,
      price: '206',
    }
    render(<TradesTable trades={[row]} formatTimeFn={formatTimeFn} activePair={mockPair} />)
    expect(screen.getByText(/1(\.0+)? UST1/)).toBeInTheDocument()
    expect(screen.getByText(/206(\.0+)? cUSTC/)).toBeInTheDocument()
    expect(screen.queryByText(/1\.000M/)).not.toBeInTheDocument()
    expect(screen.getByText('206.000')).toBeInTheDocument()
  })

  it('humanizes 18-dec USTR without compact T', () => {
    const row: IndexerTrade = {
      ...mockTrade,
      offer_asset: 'UST1',
      ask_asset: 'USTR',
      offer_amount: '1000000',
      return_amount: '10000000000000000000',
      offer_decimals: 6,
      ask_decimals: 18,
      price: '10',
    }
    render(<TradesTable trades={[row]} formatTimeFn={formatTimeFn} />)
    expect(screen.getByText(/10(\.0+)? USTR/)).toBeInTheDocument()
    expect(screen.queryByText(/\dT\b/)).not.toBeInTheDocument()
  })

  it('shows em dash when decimals are missing and pair does not match', () => {
    render(<TradesTable trades={[mockTrade]} formatTimeFn={formatTimeFn} />)
    const dashes = screen.getAllByText('—')
    expect(dashes.length).toBeGreaterThanOrEqual(2)
  })

  it('inverts tape Price but not amount in/out (GitLab #557 A7)', () => {
    const row: IndexerTrade = {
      ...mockTrade,
      offer_asset: 'UST1',
      ask_asset: 'cUSTC',
      offer_amount: '1000000',
      return_amount: '206000000',
      offer_decimals: 6,
      ask_decimals: 6,
      price: '206',
    }
    render(<TradesTable trades={[row]} formatTimeFn={formatTimeFn} activePair={mockPair} inverted />)
    expect(screen.getByText(/1(\.0+)? UST1/)).toBeInTheDocument()
    expect(screen.getByText(/206(\.0+)? cUSTC/)).toBeInTheDocument()
    expect(screen.getByText(/0\.00485/)).toBeInTheDocument()
    const pairCell = screen.getByText('UST1 → cUSTC')
    expect(pairCell).toHaveStyle({ color: 'var(--color-positive)' })
  })

  it('colors paying factory base as a sell when not inverted', () => {
    const row: IndexerTrade = {
      ...mockTrade,
      offer_asset: 'UST1',
      ask_asset: 'cUSTC',
      offer_decimals: 6,
      ask_decimals: 6,
    }
    render(<TradesTable trades={[row]} formatTimeFn={formatTimeFn} activePair={mockPair} />)
    expect(screen.getByText('UST1 → cUSTC')).toHaveStyle({ color: 'var(--color-negative)' })
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

  it('shows hybrid badge with CEX-readable tooltip when trade has pool/book split fields', () => {
    const hybridTrade: IndexerTrade = {
      ...mockTrade,
      id: 2,
      pool_return_amount: '100',
      book_return_amount: '400',
      return_amount: '500000000',
      ask_decimals: 6,
      ask_asset: 'LUNC',
    }
    render(<TradesTable trades={[hybridTrade]} formatTimeFn={formatTimeFn} />)
    const badge = screen.getByTitle(/Executed via hybrid AMM \+ limit order routing/i)
    expect(badge).toHaveTextContent('hybrid')
    expect(badge.getAttribute('title')).toMatch(/integrators\.md/)
    expect(screen.getByText(/500(\.0+)? LUNC/)).toBeInTheDocument()
  })
})
