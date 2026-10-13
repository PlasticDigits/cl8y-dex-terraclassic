import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { SwapImpactAside } from './SwapImpactAside'

describe('necessary price notices', () => {
  it('keeps an ordinary quote quiet and its optional price detail collapsed by the caller', () => {
    render(<SwapImpactAside routeSlippagePct={5} economicExplanation="Trading costs" />)
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
    expect(screen.queryByText(/Price difference/)).not.toBeInTheDocument()
  })
  it('shows one sentence, one metric and one consent action', () => {
    render(<SwapImpactAside routeSlippagePct={13.04} economicExplanation="Selected pool price movement" confirming />)
    expect(screen.getAllByRole('alert')).toHaveLength(1)
    expect(screen.getByRole('alert')).toHaveTextContent(
      'Price difference 13.04% — selected pool price movement; click again to confirm.'
    )
    expect(screen.getByRole('alert').textContent?.match(/%/g)).toHaveLength(1)
    expect(screen.getByRole('alert')).not.toHaveTextContent(/reference|breakdown|hop|Review before/i)
  })
  it.each([
    [5.000000000000001, undefined, '>5'],
    [30.000000000000004, 'expert', '>30'],
    [98.99999999999997, undefined, '<99'],
  ] as const)('does not round %s across an apparent safety boundary', (percent, blocked, expected) => {
    render(<SwapImpactAside routeSlippagePct={percent} blocked={blocked} />)
    expect(screen.getByRole('alert')).toHaveTextContent(`Price difference ${expected}%`)
    expect(screen.getByRole('alert').textContent?.match(/%/g)).toHaveLength(1)
  })
  it('never invents a cause or asks for confirmation of an extreme quote', () => {
    render(
      <SwapImpactAside
        routeSlippagePct={99}
        blocked="extreme"
        confirming
        economicExplanation="Cost breakdown unavailable."
      />
    )
    expect(screen.getByRole('alert')).toHaveTextContent('Price difference 99.00% is too high.')
    expect(screen.getByRole('alert')).not.toHaveTextContent(/confirm|cost|breakdown|expert/i)
  })
})
