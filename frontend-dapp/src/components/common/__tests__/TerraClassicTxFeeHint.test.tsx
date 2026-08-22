import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { TerraClassicTxFeeHint } from '../TerraClassicTxFeeHint'
import { MIN_GAS_PRICE_ULUNA } from '@/utils/constants'

describe('TerraClassicTxFeeHint', () => {
  it('renders retail LUNC fee without gas×price internals by default (#587)', () => {
    render(
      <TerraClassicTxFeeHint
        estimate={{
          gasLimit: 840000,
          feeUluna: 23793000n,
          gasPriceUluna: MIN_GAS_PRICE_ULUNA,
        }}
      />
    )
    const el = screen.getByTestId('terra-classic-tx-fee-hint')
    expect(el).toHaveTextContent('Network fee (est.)')
    expect(el).toHaveTextContent('LUNC')
    expect(el).not.toHaveTextContent('uluna')
    expect(el).not.toHaveTextContent('840,000 gas')
  })

  it('shows gas × price when showInternals is set', () => {
    render(
      <TerraClassicTxFeeHint
        showInternals
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
