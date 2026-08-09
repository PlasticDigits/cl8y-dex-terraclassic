import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import CreatePairPage from './CreatePairPage'
import { INVALID_TERRA_ADDRESS_CHECKSUM_MSG } from '@/utils/terraAddressValidation'

const VALID_A = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const VALID_B = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const CORRUPTED_B = `${VALID_B.slice(0, -3)}289`

vi.mock('@/hooks/useWallet', () => ({
  useWalletStore: (selector: (s: { address: string }) => unknown) => selector({ address: 'terra1wallet' }),
}))

vi.mock('@/services/terraclassic/factory', () => ({
  createPair: vi.fn(),
  getWhitelistedCodeIds: vi.fn().mockResolvedValue({ code_ids: [10] }),
}))

vi.mock('@/services/terraclassic/settings', () => ({
  getFactoryConfig: vi.fn().mockResolvedValue({ pair_creation_fee_uluna: '0' }),
}))

vi.mock('@/services/terraclassic/queries', () => ({
  getChainContractInfo: vi.fn().mockResolvedValue({ code_id: 10 }),
}))

vi.mock('@/lib/sounds', () => ({
  sounds: {
    playButtonPress: vi.fn(),
    playSuccess: vi.fn(),
    playError: vi.fn(),
  },
}))

describe('CreatePairPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('shows checksum inline error and disables submit for corrupted token B (GitLab #382)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<CreatePairPage />)

    await user.type(screen.getByLabelText(/Token A Contract Address/i), VALID_A)
    await user.type(screen.getByLabelText(/Token B Contract Address/i), CORRUPTED_B)

    expect(screen.getByText(INVALID_TERRA_ADDRESS_CHECKSUM_MSG)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Create Pair/i })).toBeDisabled()
  })

  it('notes that create pair is AMM-only, not UST1 oracle mint/redeem (GitLab #508)', () => {
    renderWithProviders(<CreatePairPage />)
    expect(screen.getByText(/UST1 mint and redeem stay on the \/ust1 oracle window/i)).toBeInTheDocument()
  })
})
