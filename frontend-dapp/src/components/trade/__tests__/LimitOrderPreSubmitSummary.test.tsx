import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { LimitOrderPreSubmitSummary } from '../LimitOrderPreSubmitSummary'

describe('LimitOrderPreSubmitSummary', () => {
  it('renders deviation and maker fee when data is ready', () => {
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
    expect(screen.getByText('15')).toBeInTheDocument()
    expect(screen.getByText('30')).toBeInTheDocument()
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
})
