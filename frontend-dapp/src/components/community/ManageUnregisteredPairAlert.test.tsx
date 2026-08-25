import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { ManageUnregisteredPairAlert } from './ManageUnregisteredPairAlert'
import { COMMUNITY_TAX_REGISTER_ALERT_COPY } from '@/utils/communityTaxRegisterPair'

const PAIR = 'terra1aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'

describe('ManageUnregisteredPairAlert (#633)', () => {
  it('shows retail copy and a single register button', () => {
    const onRegister = vi.fn()
    render(
      <MemoryRouter>
        <ManageUnregisteredPairAlert
          target={{
            pair: PAIR,
            symbols: ['QATax', 'EMBER'],
            usdTvl: 10,
            taxReserve: 1n,
            otherReserve: 2n,
          }}
          leftover={1}
          otherTokens={[{ address: 'terra1other', symbol: 'OTH' }]}
          onRegister={onRegister}
        />
      </MemoryRouter>
    )
    expect(screen.getByTestId('manage-register-alert')).toHaveTextContent(COMMUNITY_TAX_REGISTER_ALERT_COPY)
    expect(screen.getByTestId('manage-register-largest')).toHaveTextContent(/largest pool/)
    expect(screen.getByTestId('manage-register-largest')).toHaveTextContent('QATax/EMBER')
    expect(screen.queryByText(/RegisterListedPair|LISTED_PAIRS|VITE_/)).not.toBeInTheDocument()
    expect(screen.getByTestId('manage-register-leftover')).toBeInTheDocument()
    expect(screen.getByTestId('manage-register-other').querySelector('a')).toHaveAttribute(
      'href',
      '/token/terra1other/manage'
    )
    expect(screen.getByRole('link', { name: /View this pool/i })).toHaveAttribute('href', `/pool/${PAIR}`)
  })
})
