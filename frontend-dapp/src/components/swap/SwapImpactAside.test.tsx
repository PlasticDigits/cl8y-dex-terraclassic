import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { SwapImpactAside } from './SwapImpactAside'
import type { TradingRisk } from '@/utils/tradingRisk'
const risk = (patch: Partial<TradingRisk> = {}): TradingRisk => ({
  availability: 'available',
  reason: 'total',
  orderWarning: false,
  totalWarning: true,
  consentRequired: true,
  expertRequired: false,
  extreme: false,
  orderPercent: 3.34,
  totalPercent: 13.04,
  evidenceKey: 'fixture',
  ...patch,
})

describe('necessary price notices #1560', () => {
  it('keeps the 3.34 / 1.71 / 6.91 control quiet while retaining factual details', () => {
    const { rerender } = render(
      <SwapImpactAside
        routeSlippagePct={6.91}
        risk={risk({ reason: 'none', totalPercent: 6.91, totalWarning: false, consentRequired: false })}
      />
    )
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
    rerender(<SwapImpactAside routeSlippagePct={6.91} details />)
    expect(screen.getByText('Fair-reference deviation 6.91%.')).toBeInTheDocument()
  })
  it.each(['order', 'total', 'both'] as const)('shows one correctly named notice for %s', (reason) => {
    render(<SwapImpactAside routeSlippagePct={13.04} risk={risk({ reason, orderPercent: 5.01 })} confirming />)
    const status = screen.getByRole('status')
    expect(screen.getAllByRole('status')).toHaveLength(1)
    expect(status).toHaveTextContent('click again to confirm.')
    if (reason !== 'total') expect(status).toHaveTextContent('Order slippage 5.01%')
    if (reason !== 'order') expect(status).toHaveTextContent(/fair-reference deviation 13.04%/i)
  })
  it.each([
    [10.000000000000002, undefined, '>10'],
    [30.000000000000004, 'expert', '>30'],
    [98.99999999999997, undefined, '<99'],
  ] as const)('does not round %s across a safety boundary', (percent, blocked, expected) => {
    render(<SwapImpactAside routeSlippagePct={percent} risk={risk({ totalPercent: percent })} blocked={blocked} />)
    expect(screen.getByRole('status')).toHaveTextContent(`Fair-reference deviation ${expected}%`)
  })
  it('never asks for confirmation of an extreme quote', () => {
    render(
      <SwapImpactAside
        routeSlippagePct={99}
        risk={risk({ totalPercent: 99, extreme: true })}
        blocked="extreme"
        confirming
      />
    )
    expect(screen.getByRole('status')).toHaveTextContent('Fair-reference deviation 99.00% is too high.')
    expect(screen.getByRole('status')).not.toHaveTextContent(/confirm|cost|breakdown|expert/i)
  })
  it('keeps unavailable evidence textual without an Expert waiver', () => {
    render(
      <SwapImpactAside
        routeSlippagePct={0}
        risk={risk({ availability: 'unavailable' })}
        blocked="expert"
        onEnableExpert={() => {}}
      />
    )
    expect(screen.getByRole('status')).toHaveTextContent('Quote risk is unavailable. Refresh the quote.')
    expect(screen.queryByRole('button')).not.toBeInTheDocument()
  })
})
