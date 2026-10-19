import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { SwapQuoteSummary, SwapRouteDetails } from './SwapQuoteSummary'
vi.mock('@/components/ui/TokenLogo', () => ({ TokenLogo: () => <span /> }))
vi.mock('@/utils/tokenDisplay', () => ({
  getTokenDisplaySymbol: (id: string) => (id === 'uusd' ? 'USTC' : id === 'cl8y' ? 'CL8Y' : 'cUSTC'),
  getTokenLogoURI: () => undefined,
}))

describe('fitting quote summary (#1498 / #1561)', () => {
  it('retains one height while replacing unresolved values with accessible loading', () => {
    const { rerender } = render(<SwapQuoteSummary loading tokens={[]} />)
    const row = screen.getByTestId('swap-quote-summary')
    expect(row).toHaveClass('h-12')
    expect(screen.getByRole('progressbar')).toHaveAccessibleName('Loading price')
    rerender(
      <SwapQuoteSummary
        loading={false}
        tokens={['uusd', 'wrapped']}
        orderSlippagePercent={-2}
        tradingFeePercent={0.3}
      />
    )
    expect(screen.getByTestId('swap-quote-summary')).toBe(row)
    expect(screen.getByTestId('swap-order-slippage')).toHaveTextContent('-2.00%')
    expect(screen.getByTestId('swap-trading-fee')).toHaveTextContent('0.30%')
    expect(screen.getByRole('group')).not.toHaveAttribute('tabindex')
    expect(row.querySelectorAll('[data-token-identity]')).toHaveLength(2)
    expect(row.querySelector('[data-token-identity="uusd"]')).toHaveTextContent('USTC')
    expect(row.querySelector('[data-token-identity="wrapped"]')).toHaveTextContent('cUSTC')
  })
  it('narrates both legs of the selected middle-hop route with its loss and fee (#1330)', () => {
    render(
      <SwapQuoteSummary
        loading={false}
        tokens={['uusd', 'cl8y', 'wrapped']}
        orderSlippagePercent={1.25}
        tradingFeePercent={0.6}
      />
    )
    const route = screen.getByRole('group', { name: 'Selected token route' })
    expect(route).toHaveTextContent('USTC→ to CL8Y→ to cUSTC')
    expect(route.querySelectorAll('.sr-only')).toHaveLength(2)
    expect(screen.getByTestId('swap-order-slippage')).toHaveTextContent('1.25%')
    expect(screen.getByTestId('swap-trading-fee')).toHaveTextContent('0.60%')
  })
  it('never displays unavailable or subprecision metrics as zero', () => {
    const { rerender } = render(<SwapQuoteSummary loading={false} tokens={[]} />)
    expect(screen.getByTestId('swap-order-slippage')).toHaveTextContent('Unavailable')
    expect(screen.getByTestId('swap-trading-fee')).toHaveTextContent('Unavailable')
    rerender(
      <SwapQuoteSummary loading={false} tokens={[]} orderSlippagePercent={0.001} tradingFeePercent={Number.NaN} />
    )
    expect(screen.getByTestId('swap-order-slippage')).toHaveTextContent('<0.01%')
    expect(screen.getByTestId('swap-trading-fee')).toHaveTextContent('Unavailable')
  })
})

// The strip keeps every operation; DETAILS exposes identities without relying on hover.
it('retains repeated symbols, every boundary, and full unbroken identities in DETAILS', () => {
  const tokens = ['uusd', 'wrapped', 'cl8y', 'long' + 'x'.repeat(300), 'wrapped', 'uusd']
  const { rerender } = render(
    <>
      <SwapQuoteSummary loading={false} tokens={tokens} />
      <SwapRouteDetails tokens={tokens} />
    </>
  )
  const route = screen.getByRole('group', { name: 'Selected token route' })
  expect(
    Array.from(route.querySelectorAll('[data-token-identity]')).map((el) => el.getAttribute('data-token-identity'))
  ).toEqual(tokens)
  const details = screen.getByRole('list', { name: 'Full selected token route' })
  expect(Array.from(details.querySelectorAll('code')).map((el) => el.textContent)).toEqual(tokens)
  expect(route.querySelector('[data-token-identity="uusd"]')).toHaveAttribute('title', 'USTC (uusd)')
  rerender(<SwapRouteDetails tokens={[]} />)
  expect(screen.queryByTestId('swap-route-details')).not.toBeInTheDocument()
})
