import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import { useWalletStore } from '@/hooks/useWallet'
import {
  ALPHA_TERRAPORT_LUNC_PAIR,
  ALPHA_TERRAPORT_USTC_PAIR,
  MIGRATE_VENUE_CL8Y_EMPTY,
  MIGRATE_VENUE_GDEX,
} from '@/utils/communityTaxMigratePairs'
const ALPHA = 'terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz'

const { mockEnabled, getChainContractInfo, probeHasTaxMap, isCodeIdWhitelisted, loadMigratePairInventory } = vi.hoisted(
  () => ({
    mockEnabled: vi.fn(() => true),
    getChainContractInfo: vi.fn(),
    probeHasTaxMap: vi.fn(),
    isCodeIdWhitelisted: vi.fn(),
    loadMigratePairInventory: vi.fn(),
  })
)

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
  queryCommunityTaxConfig: vi.fn().mockResolvedValue({
    manager: 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3',
  }),
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

vi.mock('@/utils/communityTaxMigratePairs', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/communityTaxMigratePairs')>()
  return {
    ...actual,
    loadMigratePairInventory,
  }
})

import MigrateTokenPage from './MigrateTokenPage'

const ADMIN = 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
const OPEN = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'

const EMPTY_INV = { cl8y: [], otherDex: [], terraportIncomplete: false }
const ALPHA_INV = {
  cl8y: [],
  otherDex: [
    {
      venue: 'other_dex' as const,
      pair: ALPHA_TERRAPORT_LUNC_PAIR,
      symbols: ['ALPHA', 'LUNC'] as [string, string],
      source: 'static' as const,
    },
    {
      venue: 'other_dex' as const,
      pair: ALPHA_TERRAPORT_USTC_PAIR,
      symbols: ['ALPHA', 'USTC'] as [string, string],
      source: 'static' as const,
    },
  ],
  terraportIncomplete: false,
}

describe('MigrateTokenPage (#626 / #634 / #670)', () => {
  beforeEach(() => {
    mockEnabled.mockReturnValue(true)
    useWalletStore.setState({ address: null, walletType: null, error: null })
    getChainContractInfo.mockReset()
    probeHasTaxMap.mockReset()
    isCodeIdWhitelisted.mockReset()
    loadMigratePairInventory.mockReset()
    loadMigratePairInventory.mockResolvedValue(EMPTY_INV)
  })

  it('P0: env unset → unavailable, no invoice card, no Unlock tease', () => {
    mockEnabled.mockReturnValue(false)
    renderWithProviders(<MigrateTokenPage />)
    expect(screen.getByTestId('migrate-token-unavailable')).toBeInTheDocument()
    expect(screen.queryByTestId('pay-with-any-token')).not.toBeInTheDocument()
    expect(screen.queryByTestId('migrate-token-why')).not.toBeInTheDocument()
    expect(screen.queryByText(/Unlock 7 features/i)).not.toBeInTheDocument()
  })

  it('A9 / AC8 / #670: heading is Migrate Token, why copy shows, query params do not prefill', () => {
    renderWithProviders(<MigrateTokenPage />, {
      route: '/token/migrate?payee=terra1evil&manager=terra1evil&token=terra1evil&addr=terra1evil&pair=terra1terraport',
    })
    expect(screen.getByRole('heading', { name: /migrate token/i })).toBeInTheDocument()
    expect(screen.getByTestId('migrate-token-why')).toHaveTextContent(
      /Unlock 7 features for your token on CL8Y Dex by migrating today/i
    )
    const examples = screen.getByTestId('migrate-token-why-examples')
    expect(examples).toHaveTextContent(/buy and sell tax/i)
    expect(examples).toHaveTextContent(/Auto liquidity/)
    expect(examples).toHaveTextContent(/Launch guards/)
    expect(examples).not.toHaveTextContent(/Minting/)
    expect(screen.queryByText(/VITE_COMMUNITY_MIGRATE_CODE_IDS/)).not.toBeInTheDocument()
    expect(screen.queryByText(/allowlisted CW20/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/50 UST1/)).not.toBeInTheDocument()
    expect(screen.queryByText(/enable_feature/)).not.toBeInTheDocument()
    expect(screen.queryByText(/11619/)).not.toBeInTheDocument()
    expect(screen.queryByText(/cw2/i)).not.toBeInTheDocument()
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
    loadMigratePairInventory.mockResolvedValue(ALPHA_INV)
    renderWithProviders(<MigrateTokenPage />)
    await user.type(screen.getByTestId('migrate-token-addr'), ALPHA)
    await user.click(screen.getByTestId('migrate-token-load'))
    expect(await screen.findByTestId('migrate-token-cta')).toHaveTextContent(/free/i)
    expect(screen.getByTestId('migrate-token-confirm')).toHaveTextContent(/tax leftovers/)
    expect(screen.getByTestId('migrate-token-confirm')).toHaveTextContent(/Address stays the same/)
    expect(screen.getByTestId('migrate-token-confirm')).toHaveTextContent(/Terraport/)
    expect(screen.getByTestId('migrate-token-why')).toBeInTheDocument()
    expect(await screen.findByTestId('migrate-venue-inventory')).toBeInTheDocument()
    expect(screen.getAllByTestId('migrate-venue-other-row')).toHaveLength(2)
    expect(screen.getByText(/ALPHA\/LUNC/)).toBeInTheDocument()
    expect(screen.getByText(/ALPHA\/USTC/)).toBeInTheDocument()
    expect(screen.queryByTestId('migrate-register-cl8y')).not.toBeInTheDocument()
    expect(screen.getByTestId('migrate-venue-gdex')).toHaveTextContent(MIGRATE_VENUE_GDEX)
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
    expect(screen.getByTestId('migrate-token-confirm')).toHaveTextContent(/Address stays the same/)
    expect(await screen.findByTestId('migrate-venue-cl8y-empty')).toHaveTextContent(MIGRATE_VENUE_CL8Y_EMPTY)
    expect(screen.getByTestId('migrate-create-pair')).toHaveAttribute('href', '/create')
    expect(screen.getByTestId('migrate-venue-gdex')).toBeInTheDocument()
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
    expect(await screen.findByTestId('migrate-venue-inventory')).toBeInTheDocument()
  })

  it('AC8: ?pair= does not become a register target', async () => {
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
    renderWithProviders(<MigrateTokenPage />, { route: `/token/migrate?pair=${ALPHA_TERRAPORT_LUNC_PAIR}` })
    await user.type(screen.getByTestId('migrate-token-addr'), OPEN)
    await user.click(screen.getByTestId('migrate-token-load'))
    expect(await screen.findByTestId('migrate-token-cta')).toBeInTheDocument()
    expect(screen.queryByTestId('migrate-register-cl8y')).not.toBeInTheDocument()
    expect(screen.getByTestId('migrate-token-addr')).toHaveValue(OPEN)
  })
})
