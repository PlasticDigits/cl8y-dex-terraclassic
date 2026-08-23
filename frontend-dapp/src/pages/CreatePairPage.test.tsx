import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import CreatePairPage from './CreatePairPage'
import { createPair } from '@/services/terraclassic/factory'
import { getCreatePairCw20Options } from '@/utils/createPairTokenCatalog'
import { TOKEN_SEARCH_MAX_QUERY_LENGTH } from '@/utils/tokenSearchQuery'
import { INVALID_TERRA_ADDRESS_CHECKSUM_MSG, INVALID_TERRA_ADDRESS_FORMAT_MSG } from '@/utils/terraAddressValidation'

const VALID_A = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const VALID_B = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const CORRUPTED_B = `${VALID_B.slice(0, -3)}289`
const UNLISTED_B = 'terra1yyca08xqdgvjz0psg56z67ejh9xms6l436u8y58m82npdqqhmmtqzjqhh0'

vi.mock('react-blockies', () => ({
  __esModule: true,
  default: function MockBlockies() {
    return null
  },
}))

let walletAddress = 'terra1wallet'

vi.mock('@/hooks/useWallet', () => ({
  useWalletStore: (selector: (s: { address: string }) => unknown) => selector({ address: walletAddress }),
}))

vi.mock('@/services/terraclassic/factory', () => ({
  createPair: vi.fn().mockResolvedValue('txhash'),
  getWhitelistedCodeIds: vi.fn().mockResolvedValue({ code_ids: [10] }),
}))

vi.mock('@/services/terraclassic/settings', () => ({
  getFactoryConfig: vi.fn().mockResolvedValue({ pair_creation_fee_uluna: '0' }),
}))

vi.mock('@/services/terraclassic/queries', () => ({
  getChainContractInfo: vi.fn().mockResolvedValue({ code_id: 10 }),
  queryContract: vi.fn().mockRejectedValue(new Error('no lcd in unit test')),
}))

vi.mock('@/lib/sounds', () => ({
  sounds: {
    playButtonPress: vi.fn(),
    playSuccess: vi.fn(),
    playError: vi.fn(),
  },
}))

async function openCustom(user: ReturnType<typeof userEvent.setup>, label: 'Token A' | 'Token B') {
  const testId = `create-pair-custom-toggle-${label.replace(/\s+/g, '-').toLowerCase()}`
  await user.click(screen.getByTestId(testId))
}

async function pickListed(user: ReturnType<typeof userEvent.setup>, ariaLabel: string, address: string) {
  const combo = screen.getByRole('combobox', { name: ariaLabel })
  await user.click(combo)
  const listbox = await screen.findByRole('listbox', { name: ariaLabel })
  await user.click(within(listbox).getByTestId(`token-option-${address}`))
}

describe('CreatePairPage', () => {
  beforeEach(() => {
    walletAddress = 'terra1wallet'
    vi.clearAllMocks()
  })

  it('T15 / #508: notes that create pair is AMM-only, not UST1 oracle mint/redeem', () => {
    renderWithProviders(<CreatePairPage />)
    expect(screen.getByText(/UST1 mint and redeem stay on the \/ust1 oracle window/i)).toBeInTheDocument()
  })

  it('T10 / #382: checksum inline error and disables submit for corrupted custom token B', async () => {
    const user = userEvent.setup()
    renderWithProviders(<CreatePairPage />)

    await openCustom(user, 'Token A')
    await openCustom(user, 'Token B')
    await user.type(screen.getByLabelText(/Token A Contract Address/i), VALID_A)
    await user.type(screen.getByLabelText(/Token B Contract Address/i), CORRUPTED_B)

    expect(screen.getByText(INVALID_TERRA_ADDRESS_CHECKSUM_MSG)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Create Pair/i })).toBeDisabled()
  })

  it('T11: paste uluna into custom field is rejected', async () => {
    const user = userEvent.setup()
    renderWithProviders(<CreatePairPage />)

    await openCustom(user, 'Token A')
    await openCustom(user, 'Token B')
    await user.type(screen.getByLabelText(/Token A Contract Address/i), 'uluna')
    await user.type(screen.getByLabelText(/Token B Contract Address/i), VALID_B)

    expect(screen.getByText(INVALID_TERRA_ADDRESS_FORMAT_MSG)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Create Pair/i })).toBeDisabled()
  })

  it('T9: paste valid unlisted CW20 enables submit', async () => {
    const user = userEvent.setup()
    renderWithProviders(<CreatePairPage />)

    await openCustom(user, 'Token A')
    await openCustom(user, 'Token B')
    await user.type(screen.getByLabelText(/Token A Contract Address/i), VALID_A)
    await user.type(screen.getByLabelText(/Token B Contract Address/i), UNLISTED_B)

    expect(screen.getByRole('button', { name: /Create Pair/i })).toBeEnabled()
  })

  it('T7: pick two distinct listed tokens and submit calls createPair', async () => {
    const user = userEvent.setup()
    const [a, b] = getCreatePairCw20Options()
    expect(a && b).toBeTruthy()
    renderWithProviders(<CreatePairPage />)

    await pickListed(user, 'Select token A', a.address)
    await pickListed(user, 'Select token B', b.address)

    const submit = screen.getByRole('button', { name: /Create Pair/i })
    expect(submit).toBeEnabled()
    await user.click(submit)

    await waitFor(() => {
      expect(createPair).toHaveBeenCalledWith('terra1wallet', a.address, b.address)
    })
  })

  it('T8 / A8: same listed token or mixed-case paste disables submit', async () => {
    const user = userEvent.setup()
    const [a] = getCreatePairCw20Options()
    renderWithProviders(<CreatePairPage />)

    await pickListed(user, 'Select token A', a.address)
    await openCustom(user, 'Token B')
    await user.type(screen.getByLabelText(/Token B Contract Address/i), a.address.toUpperCase())

    expect(screen.getByText(/Token addresses must be different/i)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Create Pair/i })).toBeDisabled()
  })

  it('T12: excludeToken omits the other leg from the list', async () => {
    const user = userEvent.setup()
    const [a] = getCreatePairCw20Options()
    renderWithProviders(<CreatePairPage />)

    await pickListed(user, 'Select token A', a.address)
    await user.click(screen.getByRole('combobox', { name: 'Select token B' }))
    const listbox = await screen.findByRole('listbox', { name: 'Select token B' })
    expect(within(listbox).queryByTestId(`token-option-${a.address}`)).not.toBeInTheDocument()
  })

  it('T13: non-whitelisted code ID warns but does not disable submit', async () => {
    const { getChainContractInfo } = await import('@/services/terraclassic/queries')
    vi.mocked(getChainContractInfo).mockResolvedValue({ code_id: 99 } as never)

    const user = userEvent.setup()
    const [a, b] = getCreatePairCw20Options()
    renderWithProviders(<CreatePairPage />)

    await pickListed(user, 'Select token A', a.address)
    await pickListed(user, 'Select token B', b.address)

    await waitFor(() => {
      expect(screen.getAllByText(/Code ID 99 is not whitelisted/i).length).toBeGreaterThan(0)
    })
    expect(screen.getByText(/transaction will likely fail/i)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Create Pair/i })).toBeEnabled()
  })

  it('T14: disconnected wallet shows Connect CTA and does not submit', async () => {
    walletAddress = ''
    const user = userEvent.setup()
    renderWithProviders(<CreatePairPage />)

    const cta = screen.getByRole('button', { name: /Connect Wallet To Create/i })
    expect(cta).toBeDisabled()
    await user.click(cta)
    expect(createPair).not.toHaveBeenCalled()
  })

  it('T16: XSS-looking catalog metadata renders as text', async () => {
    const user = userEvent.setup()
    const [a] = getCreatePairCw20Options()
    localStorage.setItem(
      'cl8y-dex-token-info',
      JSON.stringify({
        [a.address.toLowerCase()]: { symbol: '<img onerror=alert(1)>', name: '<script>x</script>' },
      })
    )
    renderWithProviders(<CreatePairPage />)
    await user.click(screen.getByRole('combobox', { name: 'Select token A' }))
    const listbox = await screen.findByRole('listbox', { name: 'Select token A' })
    const option = within(listbox).getByTestId(`token-option-${a.address}`)
    expect(option.querySelector('script')).toBeNull()
    expect(option.querySelector('img[onerror]')).toBeNull()
    localStorage.removeItem('cl8y-dex-token-info')
  })

  it('T17: search paste longer than 128 chars is truncated', async () => {
    const user = userEvent.setup()
    renderWithProviders(<CreatePairPage />)
    const combo = screen.getByRole('combobox', { name: 'Select token A' })
    await user.click(combo)
    await user.paste('z'.repeat(200))
    expect((combo as HTMLInputElement).value.length).toBeLessThanOrEqual(TOKEN_SEARCH_MAX_QUERY_LENGTH)
  })

  it('P402-5 / C542-11: does not prefill Token A/B from /create query', async () => {
    const user = userEvent.setup()
    renderWithProviders(<CreatePairPage />, { route: `/create?a=${VALID_A}&b=${VALID_B}` })
    await openCustom(user, 'Token A')
    await openCustom(user, 'Token B')
    expect(screen.getByLabelText(/Token A Contract Address/i)).toHaveValue('')
    expect(screen.getByLabelText(/Token B Contract Address/i)).toHaveValue('')
    expect(screen.queryByDisplayValue(VALID_A)).not.toBeInTheDocument()
    expect(screen.queryByDisplayValue(VALID_B)).not.toBeInTheDocument()
  })

  it('C2: native LUNC / USTC / uluna are not selectable options', async () => {
    const user = userEvent.setup()
    renderWithProviders(<CreatePairPage />)
    await user.click(screen.getByRole('combobox', { name: 'Select token A' }))
    const listbox = await screen.findByRole('listbox', { name: 'Select token A' })
    const labels = within(listbox)
      .getAllByRole('option')
      .map((el) => el.textContent ?? '')
    expect(labels.some((t) => /\bLUNC\b/.test(t) && !t.includes('cLUNC'))).toBe(false)
    expect(labels.some((t) => t.includes('uluna') || t.includes('uusd'))).toBe(false)
    expect(within(listbox).queryByTestId('token-option-uluna')).not.toBeInTheDocument()
  })
})
