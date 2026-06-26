import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { LimitOrderPreSubmitSummary } from '../LimitOrderPreSubmitSummary'

describe('LimitOrderPreSubmitSummary', () => {
  it('renders deviation and retail maker fee when data is ready (#419)', () => {
    render(
      <LimitOrderPreSubmitSummary
        placeSequenceMinUluna={1500000n}
        refToken1PerToken0={2}
        typedPrice="1.8"
        effectiveFeeBps={30}
        makerPlacementFeeBps={15}
        feeLoading={false}
        feeError={false}
      />
    )
    expect(screen.getByText(/no taker slippage/i)).toBeInTheDocument()
    expect(screen.getByText('-10.0%')).toBeInTheDocument()
    const feeLine = screen.getByTestId('limit-order-pre-submit-summary-maker-fee')
    expect(feeLine).toHaveTextContent(/Small fee taken from your escrow at placement/i)
    expect(feeLine).toHaveTextContent(/0\.15%/)
    expect(feeLine).toHaveTextContent(/15/)
    expect(feeLine).toHaveTextContent(/0\.30%/)
  })

  it('shows loading state for maker fee while queries run', () => {
    render(
      <LimitOrderPreSubmitSummary
        placeSequenceMinUluna={0n}
        refToken1PerToken0={1}
        typedPrice="1"
        effectiveFeeBps={null}
        makerPlacementFeeBps={null}
        feeLoading
        feeError={false}
      />
    )
    expect(screen.getByText('Loading…')).toBeInTheDocument()
  })

  it('links to limit order fee docs without issue refs', () => {
    render(
      <LimitOrderPreSubmitSummary
        placeSequenceMinUluna={1500000n}
        refToken1PerToken0={2}
        typedPrice="1.8"
        effectiveFeeBps={30}
        makerPlacementFeeBps={15}
        feeLoading={false}
        feeError={false}
      />
    )
    const link = screen.getByRole('link', { name: /Learn more about limit order fees/i })
    expect(link.getAttribute('href')).toContain('limit-orders.md')
    expect(screen.queryByText(/GitLab #/i)).not.toBeInTheDocument()
  })
})
