import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import MintPage from './MintPage'
import { useWalletStore } from '@/hooks/useWallet'
import * as faucetService from '@/services/terraclassic/faucet'

const { FAUCET, WALLET, EMBER, CORAL, mockIsFaucetEnabled } = vi.hoisted(() => ({
  FAUCET: 'terra1faucet000000000000000000000000000000001',
  WALLET: 'terra1wallet00000000000000000000000000001',
  EMBER: 'terra1ember00000000000000000000000000000001',
  CORAL: 'terra1coral00000000000000000000000000000002',
  mockIsFaucetEnabled: vi.fn(() => true),
}))

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn(), playSuccess: vi.fn(), playError: vi.fn() },
}))

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    FAUCET_CONTRACT_ADDRESS: FAUCET,
    isFaucetEnabled: () => mockIsFaucetEnabled(),
    SOFT_LAUNCH_MINTABLE_TOKENS: [
      { symbol: 'EMBER', address: EMBER, decimals: 6 },
      { symbol: 'CORAL', address: CORAL, decimals: 6 },
    ],
  }
})

vi.mock('@/services/terraclassic/faucet', () => ({
  getFaucetConfig: vi.fn(),
  getFaucetCooldown: vi.fn(),
  drip: vi.fn(),
}))

describe('MintPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockIsFaucetEnabled.mockReturnValue(true)
    useWalletStore.setState({ address: null, walletType: null, error: null })
    vi.mocked(faucetService.getFaucetConfig).mockResolvedValue({
      admin: 'terra1admin',
      drip_amount: '100000000',
      cooldown_seconds: 300,
      paused: false,
      allowed_tokens: [EMBER, CORAL],
    })
    vi.mocked(faucetService.getFaucetCooldown).mockResolvedValue({
      can_claim: true,
      seconds_remaining: 0,
      last_claim_at: null,
      paused: false,
    })
  })

  it('shows unavailable when faucet is not configured', () => {
    mockIsFaucetEnabled.mockReturnValue(false)
    renderWithProviders(<MintPage />)
    expect(screen.getByTestId('mint-unavailable')).toHaveTextContent(/not available/i)
  })

  it('shows demo disclaimer and mintable tokens when faucet is enabled', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    renderWithProviders(<MintPage />)

    expect(await screen.findByText(/demo only/i)).toBeInTheDocument()
    expect(screen.getByText(/no economic value/i)).toBeInTheDocument()
    expect(await screen.findByTestId('mint-token-select')).toBeInTheDocument()
    expect(await screen.findByText('100')).toBeInTheDocument()
    expect(screen.getByText(/network gas/i)).toBeInTheDocument()
  })

  it('blocks mint when cooldown is active', async () => {
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    vi.mocked(faucetService.getFaucetCooldown).mockResolvedValue({
      can_claim: false,
      seconds_remaining: 185,
      last_claim_at: '1700000000000000000',
      paused: false,
    })

    renderWithProviders(<MintPage />)

    await screen.findByTestId('mint-cooldown')
    expect(screen.getByTestId('mint-cooldown')).toHaveTextContent(/3:05/)
    expect(screen.getByTestId('mint-submit')).toBeDisabled()
  })

  it('submits drip when ready', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    vi.mocked(faucetService.drip).mockResolvedValue('txhash123')

    renderWithProviders(<MintPage />)

    const submit = await screen.findByTestId('mint-submit')
    await waitFor(() => expect(submit).toBeEnabled())
    await user.click(submit)

    await waitFor(() => {
      expect(faucetService.drip).toHaveBeenCalledWith(WALLET, EMBER)
    })
    expect(await screen.findByText(/mint successful/i)).toBeInTheDocument()
  })
})
