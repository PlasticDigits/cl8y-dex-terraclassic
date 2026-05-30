import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { TerraClassicTxFeeHint } from '../TerraClassicTxFeeHint'
import { MIN_GAS_PRICE_ULUNA } from '@/utils/constants'

describe('TerraClassicTxFeeHint', () => {
  it('renders gas limit and Classic gas price', () => {
    render(
      <TerraClassicTxFeeHint
        estimate={{
          gasLimit: 840000,
          feeUluna: 23793000n,
          gasPriceUluna: MIN_GAS_PRICE_ULUNA,
        }}
      />
    )
    expect(screen.getByTestId('terra-classic-tx-fee-hint')).toHaveTextContent('840,000 gas')
    expect(screen.getByTestId('terra-classic-tx-fee-hint')).toHaveTextContent(String(MIN_GAS_PRICE_ULUNA))
  })
})
