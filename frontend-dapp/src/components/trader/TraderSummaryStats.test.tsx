import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { TraderSummaryStats } from './TraderSummaryStats'
import type { IndexerTrader } from '@/types'

vi.mock('@/lib/sounds', () => ({
  sounds: {
    playButtonPress: vi.fn(),
    playHover: vi.fn(),
    playSuccess: vi.fn(),
    playError: vi.fn(),
  },
}))

const ADDR = 'terra1abcdefghijklmnopqrstuvwxyz1234567890abcd'

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
    total_realized_pnl: '0',
    best_trade_pnl: null,
    worst_trade_pnl: null,
    total_fees_paid: '0',
    ...overrides,
  }
}

function renderStats(row: IndexerTrader) {
  return render(
    <MemoryRouter>
      <TraderSummaryStats trader={row} />
    </MemoryRouter>
  )
}

describe('TraderSummaryStats Total Volume (GitLab #553)', () => {
  it('formats USD compact and does not print raw USTR-scale T', () => {
    renderStats(trader())
    const box = screen.getByTestId('trader-total-volume-usd')
    expect(box).toHaveTextContent(/total volume \(usd\)/i)
    expect(box.textContent).toMatch(/\$/)
    expect(box.textContent).not.toMatch(/10,000,000T/)
    expect(box.textContent).not.toMatch(/\dT\b/)
  })

  it('unpriced volume with trades is an em dash, not $0', () => {
    renderStats(trader({ total_volume_usd: null, total_trades: 4 }))
    const box = screen.getByTestId('trader-total-volume-usd')
    expect(box).toHaveTextContent('—')
    expect(box.textContent).not.toMatch(/\$0/)
  })

  it('zero trades is $0', () => {
    renderStats(trader({ total_trades: 0, total_volume: '0', total_volume_usd: '0' }))
    expect(screen.getByTestId('trader-total-volume-usd')).toHaveTextContent('$0')
  })
})
