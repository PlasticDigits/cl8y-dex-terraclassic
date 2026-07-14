import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { LimitOrderPreSubmitSummary } from '../LimitOrderPreSubmitSummary'

const baseProps = {
  pairLabel: 'EMBER / CORAL',
  sideLabel: 'Buy EMBER',
  escrowAmountLabel: '12.5 CORAL',
  placeSequenceMinUluna: 1500000n,
  refToken1PerToken0: 2,
  typedPrice: '1.8',
  effectiveFeeBps: 30,
  makerPlacementFeeBps: 15,
  feeLoading: false,
  feeError: false,
}

describe('LimitOrderPreSubmitSummary', () => {
  it('renders deviation and compact maker fee when data is ready (#419 / #488)', () => {
    render(<LimitOrderPreSubmitSummary {...baseProps} />)
    expect(screen.getByTestId('limit-order-pre-submit-summary-vs-ref')).toHaveTextContent('-10.0%')
    const feeLine = screen.getByTestId('limit-order-pre-submit-summary-maker-fee')
    expect(feeLine).toHaveTextContent(/0\.15%/)
  })

  // GitLab #461 (SEC-I05 F-02): labeled signing fields before the wallet dialog.
  it('renders action, pair, side, amount, and chain label for anti-phishing', () => {
    render(<LimitOrderPreSubmitSummary {...baseProps} chainFullLabel="Terra Classic" />)
    expect(screen.getByTestId('limit-order-pre-submit-summary-action')).toHaveTextContent('Place Limit Order')
    expect(screen.getByTestId('limit-order-pre-submit-summary-pair')).toHaveTextContent('EMBER / CORAL')
    expect(screen.getByTestId('limit-order-pre-submit-summary-side')).toHaveTextContent('Buy EMBER')
    expect(screen.getByTestId('limit-order-pre-submit-summary-amount')).toHaveTextContent('12.5 CORAL')
    const chainRow = screen.getByTestId('limit-order-pre-submit-summary-chain')
    expect(chainRow).toHaveTextContent('Terra Classic')
  })

  it('falls back to the env network badge when no chain label is passed', () => {
    render(<LimitOrderPreSubmitSummary {...baseProps} />)
    expect(screen.getByTestId('limit-order-pre-submit-summary-chain').textContent?.trim().length).toBeGreaterThan(0)
  })

  it('shows loading state for maker fee while queries run', () => {
    render(
      <LimitOrderPreSubmitSummary
        {...baseProps}
        placeSequenceMinUluna={0n}
        refToken1PerToken0={1}
        typedPrice="1"
        effectiveFeeBps={null}
        makerPlacementFeeBps={null}
        feeLoading
      />
    )
    expect(screen.getByText('…')).toBeInTheDocument()
  })

  it('links to limit order docs without issue refs', () => {
    render(<LimitOrderPreSubmitSummary {...baseProps} />)
    const link = screen.getByRole('link', { name: /^Docs$/i })
    expect(link.getAttribute('href')).toContain('limit-orders.md')
    expect(screen.queryByText(/GitLab #/i)).not.toBeInTheDocument()
  })
})
