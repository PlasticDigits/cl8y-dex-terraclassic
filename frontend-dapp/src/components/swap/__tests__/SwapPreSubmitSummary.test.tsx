import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { SwapPreSubmitSummary } from '../SwapPreSubmitSummary'

describe('SwapPreSubmitSummary (GitLab #409 / SEC-D11)', () => {
  it('renders labeled action, pair, amounts, max spread, min return, and chain', () => {
    render(
      <SwapPreSubmitSummary
        offerSymbol="AAA"
        receiveSymbol="BBB"
        offerAmountHuman="1.5"
        receiveAmountHuman="0.99"
        maxSpreadPercent={0.5}
        minReceiveHuman="0.98"
        chainFullLabel="LocalTerra"
      />
    )

    expect(screen.getByTestId('swap-pre-submit-summary')).toBeInTheDocument()
    expect(screen.getByTestId('swap-confirm-action')).toHaveTextContent('Swap')
    expect(screen.getByTestId('swap-confirm-pair')).toHaveTextContent('AAA → BBB')
    expect(screen.getByTestId('swap-confirm-offer')).toHaveTextContent('1.5 AAA')
    expect(screen.getByTestId('swap-confirm-receive')).toHaveTextContent('0.99 BBB')
    expect(screen.getByTestId('swap-confirm-max-spread')).toHaveTextContent('0.5%')
    expect(screen.getByTestId('swap-confirm-min-return')).toHaveTextContent('0.98 BBB')
    expect(screen.getByTestId('swap-confirm-chain')).toHaveTextContent('LocalTerra')
  })

  it('supports custom action labels for market swaps', () => {
    render(
      <SwapPreSubmitSummary
        actionLabel="Market swap"
        offerSymbol="LUNC"
        receiveSymbol="CL8Y"
        offerAmountHuman="100"
        receiveAmountHuman="50"
        maxSpreadPercent={1}
        minReceiveHuman="49.5"
        chainFullLabel="Terra Classic"
        data-testid="trade-market-pre-submit-summary"
      />
    )

    expect(screen.getByTestId('trade-market-pre-submit-summary')).toBeInTheDocument()
    expect(screen.getByTestId('swap-confirm-action')).toHaveTextContent('Market swap')
    expect(screen.getByTestId('swap-confirm-chain')).toHaveTextContent('Terra Classic')
  })
})
