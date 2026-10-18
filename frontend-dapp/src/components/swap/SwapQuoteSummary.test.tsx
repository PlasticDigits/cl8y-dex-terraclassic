import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { SwapQuoteSummary } from './SwapQuoteSummary'
vi.mock('@/components/ui/TokenLogo', () => ({ TokenLogo: () => <span /> }))
vi.mock('@/utils/tokenDisplay', () => ({
  getTokenDisplaySymbol: (id: string) => (id === 'uusd' ? 'USTC' : id === 'cl8y' ? 'CL8Y' : 'cUSTC'),
  getTokenLogoURI: () => undefined,
}))

describe('fixed quote summary (#1498)', () => {
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
    expect(screen.getByRole('group')).toHaveAttribute('tabindex', '0')
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
