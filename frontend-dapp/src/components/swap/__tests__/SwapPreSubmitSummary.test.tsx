import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { SwapPreSubmitSummary } from '../SwapPreSubmitSummary'
import { shortenAddress } from '@/utils/tokenDisplay'

const DIRECT_PAIR = 'terra1pairabababababababababababababababab'
const HOP_PAIR_0 = 'terra1pair00000000000000000000000000000001'
const HOP_PAIR_1 = 'terra1pair11111111111111111111111111111111'

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

  it('renders factory-sourced pair contract address for direct swaps (#449 / SEC-I02)', () => {
    render(
      <SwapPreSubmitSummary
        offerSymbol="AAA"
        receiveSymbol="BBB"
        offerAmountHuman="1"
        receiveAmountHuman="0.99"
        maxSpreadPercent={0.5}
        minReceiveHuman="0.98"
        pairContractAddresses={[DIRECT_PAIR]}
      />
    )

    expect(screen.getByTestId('swap-confirm-pair-contracts')).toHaveTextContent('Pair contract')
    expect(screen.getByTestId('swap-confirm-pair-contract')).toBeInTheDocument()
    expect(screen.getByText(shortenAddress(DIRECT_PAIR))).toBeInTheDocument()
  })

  it('renders hop pair contract addresses for multihop routes (#449)', () => {
    render(
      <SwapPreSubmitSummary
        offerSymbol="AAA"
        receiveSymbol="CCC"
        offerAmountHuman="1"
        receiveAmountHuman="0.5"
        maxSpreadPercent={1}
        minReceiveHuman="0.49"
        pairContractAddresses={[HOP_PAIR_0, HOP_PAIR_1]}
      />
    )

    expect(screen.getByTestId('swap-confirm-pair-contracts')).toHaveTextContent('Pair contracts')
    expect(screen.getByTestId('swap-confirm-hop-pair-0')).toBeInTheDocument()
    expect(screen.getByTestId('swap-confirm-hop-pair-1')).toBeInTheDocument()
    expect(screen.getByText(shortenAddress(HOP_PAIR_0, 6, 4))).toBeInTheDocument()
    expect(screen.getByText(shortenAddress(HOP_PAIR_1, 6, 4))).toBeInTheDocument()
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
