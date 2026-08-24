import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import { useWalletStore } from '@/hooks/useWallet'
const ALPHA = 'terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz'

const { mockEnabled, getChainContractInfo, probeHasTaxMap, isCodeIdWhitelisted } = vi.hoisted(() => ({
  mockEnabled: vi.fn(() => true),
  getChainContractInfo: vi.fn(),
  probeHasTaxMap: vi.fn(),
  isCodeIdWhitelisted: vi.fn(),
}))

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn(), playSuccess: vi.fn(), playError: vi.fn() },
}))

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    COMMUNITY_TAX_CODE_ID: 11619,
    COMMUNITY_MIGRATE_CODE_IDS: [6036, 10184, 8266, 8654],
    COMMUNITY_TOKEN_LAUNCHER: 'terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze',
    DEFAULT_NETWORK: 'mainnet',
    isCommunityTaxEnabled: () => mockEnabled(),
  }
})

vi.mock('@/services/terraclassic/queries', () => ({
  getChainContractInfo,
}))

vi.mock('@/services/terraclassic/factory', () => ({
  isCodeIdWhitelisted,
}))

vi.mock('@/services/terraclassic/communityTaxToken', () => ({
  probeHasTaxMap,
  queryCommunityTaxTokenInfo: vi.fn().mockResolvedValue({ name: 'Open', symbol: 'Open', decimals: 6 }),
  queryLauncherConfig: vi.fn().mockResolvedValue({
    token_code_id: 11619,
    autolp_code_id: 11621,
    ust1: 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
    cmm_treasury: 'terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2',
    cmm_governance: 'terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2',
    factory: 'terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea',
    router: null,
  }),
  migrateAdoptCommunityToken: vi.fn().mockResolvedValue('migrate-hash'),
}))

import MigrateTokenPage from './MigrateTokenPage'

const ADMIN = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const OPEN = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'

describe('MigrateTokenPage (#626)', () => {
  beforeEach(() => {
    mockEnabled.mockReturnValue(true)
    useWalletStore.setState({ address: null, walletType: null, error: null })
    getChainContractInfo.mockReset()
    probeHasTaxMap.mockReset()
    isCodeIdWhitelisted.mockReset()
  })

  it('P0: env unset → unavailable, no invoice card', () => {
    mockEnabled.mockReturnValue(false)
    renderWithProviders(<MigrateTokenPage />)
    expect(screen.getByTestId('migrate-token-unavailable')).toBeInTheDocument()
    expect(screen.queryByTestId('pay-with-any-token')).not.toBeInTheDocument()
  })

  it('A9: heading is Migrate Token and no invoice', () => {
    renderWithProviders(<MigrateTokenPage />, { route: '/token/migrate?payee=terra1evil&manager=terra1evil' })
    expect(screen.getByRole('heading', { name: /migrate token/i })).toBeInTheDocument()
    expect(screen.getByText(/move an existing token onto this template/i)).toBeInTheDocument()
    expect(screen.queryByText(/VITE_COMMUNITY_MIGRATE_CODE_IDS/)).not.toBeInTheDocument()
    expect(screen.queryByText(/allowlisted CW20/i)).not.toBeInTheDocument()
    expect(screen.queryByTestId('pay-with-any-token')).not.toBeInTheDocument()
    expect(screen.getByTestId('migrate-token-addr')).toHaveValue('')
  })

  it('P6: allowlisted 8654 + wasm admin shows free wipe CTA', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: ADMIN, walletType: 'keplr', error: null })
    getChainContractInfo.mockResolvedValue({
      code_id: 8654,
      admin: ADMIN,
      creator: ADMIN,
      label: 'alpha',
    })
    probeHasTaxMap.mockResolvedValue(true)
    renderWithProviders(<MigrateTokenPage />)
    await user.type(screen.getByTestId('migrate-token-addr'), ALPHA)
    await user.click(screen.getByTestId('migrate-token-load'))
    expect(await screen.findByTestId('migrate-token-cta')).toHaveTextContent(/free/i)
    expect(screen.getByTestId('migrate-token-confirm')).toHaveTextContent(/tax leftovers/)
    expect(screen.getByTestId('migrate-token-confirm')).toHaveTextContent(/Terraport/)
    expect(screen.queryByTestId('pay-with-any-token')).not.toBeInTheDocument()
  })

  it('P3: listed 10184 + wasm admin shows free CTA', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: ADMIN, walletType: 'keplr', error: null })
    getChainContractInfo.mockResolvedValue({
      code_id: 10184,
      admin: ADMIN,
      creator: ADMIN,
      label: 'mintable',
    })
    probeHasTaxMap.mockResolvedValue(false)
    isCodeIdWhitelisted.mockResolvedValue({ code_id: 10184, whitelisted: true })
    renderWithProviders(<MigrateTokenPage />)
    await user.type(screen.getByTestId('migrate-token-addr'), OPEN)
    await user.click(screen.getByTestId('migrate-token-load'))
    expect(await screen.findByTestId('migrate-token-cta')).toHaveTextContent(/free/i)
    expect(screen.getByTestId('migrate-token-confirm')).toHaveTextContent(/Terraport/)
    expect(screen.queryByTestId('pay-with-any-token')).not.toBeInTheDocument()
  })

  it('A10: holder wallet hides the button', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({
      address: 'terra1yyca08xqdgvjz0psg56z67ejh9xms6l436u8y58m82npdqqhmmtqzjqhh0',
      walletType: 'keplr',
      error: null,
    })
    getChainContractInfo.mockResolvedValue({
      code_id: 6036,
      admin: ADMIN,
      creator: ADMIN,
      label: 'base',
    })
    probeHasTaxMap.mockResolvedValue(false)
    isCodeIdWhitelisted.mockResolvedValue({ code_id: 6036, whitelisted: true })
    renderWithProviders(<MigrateTokenPage />)
    await user.type(screen.getByTestId('migrate-token-addr'), OPEN)
    await user.click(screen.getByTestId('migrate-token-load'))
    expect(await screen.findByTestId('migrate-token-verdict-not_admin')).toBeInTheDocument()
    expect(screen.queryByTestId('migrate-token-cta')).not.toBeInTheDocument()
  })
})
