import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { RouteOrderCostDetails } from './RouteOrderCostDetails'

describe('canonical signed final-output costs', () => {
  it.each([6, 18, 0])('formats favorable raw reference amounts truthfully at %s decimals', (decimals) => {
    const raw = decimals === 0 ? '-123' : `-${123n * 10n ** BigInt(decimals) + 1n}`
    render(
      <RouteOrderCostDetails
        symbol="OUT"
        decimals={decimals}
        costs={{ tradingFeeRaw: '0', taxHookRaw: '0', wrapperRaw: '0', referenceDivergenceRaw: raw }}
      />
    )
    expect(screen.getByTestId('route-order-cost-details')).toHaveTextContent(
      decimals === 0 ? '-123 OUT' : `-123.${'0'.repeat(decimals - 1)}1 OUT`
    )
    expect(screen.getByTestId('route-order-cost-details')).toHaveTextContent(`${raw} raw`)
    expect(screen.getByTestId('route-order-cost-details')).not.toHaveTextContent(/\.0+-/)
  })
  it('keeps missing diagnostic costs unavailable', () => {
    render(<RouteOrderCostDetails symbol="OUT" decimals={6} costs={null} />)
    expect(screen.getByText('Cost breakdown unavailable.')).toBeInTheDocument()
  })
})
