import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import { useWalletStore } from '@/hooks/useWallet'

const { mockEnabled } = vi.hoisted(() => ({ mockEnabled: vi.fn(() => true) }))

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn(), playSuccess: vi.fn(), playError: vi.fn() },
}))

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    COMMUNITY_TAX_CODE_ID: 11611,
    COMMUNITY_TOKEN_LAUNCHER: 'terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze',
    UST1_TOKEN_ADDRESS: 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
    isCommunityTaxEnabled: () => mockEnabled(),
  }
})

vi.mock('@/services/indexer/client', () => ({
  getTokens: vi.fn().mockResolvedValue([]),
}))

vi.mock('@/services/terraclassic/communityTaxToken', () => ({
  createFreeCommunityToken: vi.fn().mockResolvedValue('create-tx-hash'),
}))

vi.mock('@/components/payments/PayWithAnyToken', () => ({
  PayWithAnyToken: ({ invoice }: { invoice: { invoiceAmount: string; payee: string } }) => (
    <div data-testid="pay-with-any-token">
      {invoice.invoiceAmount}:{invoice.payee}
    </div>
  ),
}))

import CreateTokenPage from './CreateTokenPage'

const WALLET = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'

describe('CreateTokenPage (#593)', () => {
  beforeEach(() => {
    mockEnabled.mockReturnValue(true)
    useWalletStore.setState({ address: null, walletType: null, error: null })
  })

  it('shows unavailable when launcher env is unset', () => {
    mockEnabled.mockReturnValue(false)
    renderWithProviders(<CreateTokenPage />)
    expect(screen.getByTestId('create-token-unavailable')).toBeInTheDocument()
  })

  it('SKU toggle math 0 / 1 / 3 → 0 / 50 / 150 UST1', async () => {
    const user = userEvent.setup()
    renderWithProviders(<CreateTokenPage />)
    expect(screen.getByTestId('create-token-sku-total')).toHaveTextContent('0 UST1')
    await user.click(screen.getByTestId('create-token-sku-transfer_tax'))
    expect(screen.getByTestId('create-token-sku-total')).toHaveTextContent('50 UST1')
    await user.click(screen.getByTestId('create-token-sku-split_router'))
    await user.click(screen.getByTestId('create-token-sku-variable_rates'))
    expect(screen.getByTestId('create-token-sku-total')).toHaveTextContent('150 UST1')
  })

  it('disconnected create is gated (form still visible)', () => {
    renderWithProviders(<CreateTokenPage />)
    expect(screen.getByTestId('create-token-connect')).toBeInTheDocument()
    expect(screen.queryByTestId('create-token-free-cta')).not.toBeInTheDocument()
  })

  it('does not confuse copy with faucet Mint or Create Pair heading', () => {
    renderWithProviders(<CreateTokenPage />)
    expect(screen.getByRole('heading', { name: /create token/i })).toBeInTheDocument()
    expect(screen.getByText(/not the DEX swap fee/i)).toBeInTheDocument()
    expect(screen.getByText(/CMM-only/i)).toBeInTheDocument()
  })

  it('paid SKUs render PayWithAnyToken targeting the launcher', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: WALLET, walletType: 'keplr', error: null })
    renderWithProviders(<CreateTokenPage />)
    await user.type(screen.getByTestId('create-token-name'), 'Demo')
    await user.type(screen.getByTestId('create-token-symbol'), 'DEMO')
    await user.click(screen.getByTestId('create-token-ack'))
    await user.click(screen.getByTestId('create-token-sku-transfer_tax'))
    expect(await screen.findByTestId('pay-with-any-token')).toHaveTextContent(
      '50000000:terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze'
    )
  })

  it('P402-5: after free create, Create Pair link is /create with no query prefill', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: WALLET, walletType: 'keplr', error: null })
    renderWithProviders(<CreateTokenPage />)
    await user.type(screen.getByTestId('create-token-name'), 'Demo')
    await user.type(screen.getByTestId('create-token-symbol'), 'DEMO')
    await user.click(screen.getByTestId('create-token-ack'))
    await user.click(screen.getByTestId('create-token-free-cta'))
    const next = await screen.findByTestId('create-token-next-create-pair')
    expect(next).toHaveAttribute('href', '/create')
    expect(next.getAttribute('href')).not.toMatch(/\?/)
  })
})
