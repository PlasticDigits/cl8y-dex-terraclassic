import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PoolPreSubmitSummary } from '../PoolPreSubmitSummary'

// GitLab #462 (SEC-I05 F-03): pool provide/withdraw must show an in-app pre-sign card with
// action, pair, amounts, and chain label before the wallet dialog — the SEC-D11 anti-phishing
// anchor swaps already have.
describe('PoolPreSubmitSummary', () => {
  it('renders provide action, pair, consolidated deposit amounts, and chain label', () => {
    render(
      <PoolPreSubmitSummary
        actionLabel="Provide Liquidity"
        pairLabel="EMBER / CORAL"
        amountLines={['12.5 EMBER', '8.0 CORAL']}
        chainFullLabel="Terra Classic"
        data-testid="pool-provide-pre-submit-summary"
      />
    )
    expect(screen.getByTestId('pool-provide-pre-submit-summary-action')).toHaveTextContent('Provide Liquidity')
    expect(screen.getByTestId('pool-provide-pre-submit-summary-pair')).toHaveTextContent('EMBER / CORAL')
    expect(screen.getByTestId('pool-provide-pre-submit-summary-amount')).toHaveTextContent('12.5 EMBER + 8.0 CORAL')
    expect(screen.getByTestId('pool-provide-pre-submit-summary-chain')).toHaveTextContent('Terra Classic')
  })

  it('renders withdraw action with a single LP amount line', () => {
    render(
      <PoolPreSubmitSummary
        actionLabel="Withdraw Liquidity"
        pairLabel="EMBER / CORAL"
        amountLines={['3.0 LP']}
        chainFullLabel="Terra Classic"
        data-testid="pool-withdraw-pre-submit-summary"
      />
    )
    expect(screen.getByTestId('pool-withdraw-pre-submit-summary-action')).toHaveTextContent('Withdraw Liquidity')
    expect(screen.getByTestId('pool-withdraw-pre-submit-summary-amount')).toHaveTextContent('3.0 LP')
    expect(screen.getByTestId('pool-withdraw-pre-submit-summary-chain')).toHaveTextContent('Terra Classic')
  })

  it('falls back to the env network badge when no chain label is passed', () => {
    render(<PoolPreSubmitSummary actionLabel="Provide Liquidity" pairLabel="A / B" amountLines={['1 A', '1 B']} />)
    expect(screen.getByTestId('pool-pre-submit-summary-chain').textContent?.trim().length).toBeGreaterThan(0)
  })
})
