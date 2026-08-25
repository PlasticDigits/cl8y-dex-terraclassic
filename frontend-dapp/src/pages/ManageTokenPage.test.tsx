import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryRouter, RouterProvider } from 'react-router-dom'
import { useWalletStore } from '@/hooks/useWallet'
import { isManagerWallet } from '@/utils/communityTaxManager'

const { mockEnabled, TOKEN, MANAGER, OTHER } = vi.hoisted(() => ({
  mockEnabled: vi.fn(() => true),
  TOKEN: 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v',
  MANAGER: 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3',
  OTHER: 'terra1yyca08xqdgvjz0psg56z67ejh9xms6l436u8y58m82npdqqhmmtqzjqhh0',
}))

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
    CMM_GOVERNANCE_ADDR: 'terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2',
    isCommunityTaxEnabled: () => mockEnabled(),
  }
})

vi.mock('@/services/indexer/client', () => ({
  getTokens: vi.fn().mockResolvedValue([]),
  getTokenPairs: vi.fn().mockResolvedValue([]),
  getCommunityTokens: vi.fn().mockResolvedValue({ items: [] }),
  getHubPrices: vi.fn().mockResolvedValue({ prices: [] }),
}))

vi.mock('@/services/terraclassic/queries', () => ({
  getChainContractInfo: vi.fn().mockResolvedValue({
    code_id: 11611,
    admin: 'terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2',
    creator: MANAGER,
    label: 'community-tax-DEMO',
  }),
}))

vi.mock('@/services/terraclassic/communityTaxToken', () => ({
  queryCommunityTaxConfig: vi.fn().mockResolvedValue({
    manager: MANAGER,
    treasury: MANAGER,
    buy_bps: 100,
    sell_bps: 100,
    transfer_bps: 0,
    max_buy_bps: 1000,
    max_sell_bps: 1000,
    max_transfer_bps: 500,
    factory: MANAGER,
    router: null,
    ust1: MANAGER,
    cmm_treasury: MANAGER,
    autolp: null,
    sinks: [],
    launch_guards: null,
    mint_revoked: false,
  }),
  queryCommunityTaxFeatures: vi.fn().mockResolvedValue({
    mint_control: false,
    transfer_tax: false,
    split_router: false,
    auto_v2_lp: false,
    exemption_directory: false,
    variable_rates: true,
    launch_guards: false,
  }),
  mintCommunityTax: vi.fn(),
  skimAutoLp: vi.fn(),
  registerListedPair: vi.fn(),
  queryCommunityTaxIsExempt: vi.fn().mockResolvedValue({ address: '', protocol: true, manager: false }),
  queryCommunityTaxTokenInfo: vi.fn().mockResolvedValue({ name: 'Demo', symbol: 'DEMO', decimals: 6 }),
}))

vi.mock('@/utils/communityTaxRegisterPair', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/communityTaxRegisterPair')>()
  return {
    ...actual,
    loadUnregisteredFactoryPairs: vi.fn().mockResolvedValue([]),
    tokensNeedingRegisterForManager: vi.fn().mockResolvedValue([]),
  }
})

vi.mock('@/components/payments/PayWithAnyToken', () => ({
  PayWithAnyToken: ({ invoice }: { invoice: { invoiceAmount: string } }) => (
    <div data-testid="pay-with-any-token">{invoice.invoiceAmount}</div>
  ),
}))

import ManageTokenPage from './ManageTokenPage'

function renderManage(wallet: string | null) {
  useWalletStore.setState({ address: wallet, walletType: wallet ? 'keplr' : null, error: null })
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  })
  const router = createMemoryRouter([{ path: '/token/:addr/manage', element: <ManageTokenPage /> }], {
    initialEntries: [`/token/${TOKEN}/manage`],
  })
  return render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  )
}

describe('ManageTokenPage (#593)', () => {
  beforeEach(async () => {
    mockEnabled.mockReturnValue(true)
    const { getChainContractInfo } = await import('@/services/terraclassic/queries')
    vi.mocked(getChainContractInfo).mockResolvedValue({
      code_id: 11611,
      admin: 'terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2',
      creator: MANAGER,
      label: 'community-tax-DEMO',
    })
  })

  it('compares manager to the connected wallet, not a URL param', () => {
    expect(isManagerWallet(MANAGER, MANAGER)).toBe(true)
    expect(isManagerWallet(OTHER, MANAGER)).toBe(false)
    expect(isManagerWallet(null, MANAGER)).toBe(false)
  })

  it('non-manager sees config but cannot save', async () => {
    renderManage(OTHER)
    expect(await screen.findByTestId('manage-readonly')).toBeInTheDocument()
    expect(screen.getByTestId('manage-save-disabled')).toBeDisabled()
    expect(screen.getByTestId('manage-save-copy')).toHaveTextContent('50 UST1')
  })

  it('discloses pair-direct buy/sell tax (#607)', async () => {
    renderManage(MANAGER)
    expect(await screen.findByTestId('manage-token-tax-scope')).toHaveTextContent(
      'Buy/sell tax applies on every listed-pair swap.'
    )
  })

  it('Minting SKU is not in the unlock list', async () => {
    renderManage(MANAGER)
    expect(await screen.findByTestId('manage-unlock-sku')).toBeInTheDocument()
    expect(screen.getByTestId('manage-unlock-sku').textContent).not.toMatch(/Minting/)
  })

  it('M-1: buy/sell stay locked without variable_rates', async () => {
    const { queryCommunityTaxFeatures } = await import('@/services/terraclassic/communityTaxToken')
    vi.mocked(queryCommunityTaxFeatures).mockResolvedValueOnce({
      mint_control: false,
      transfer_tax: false,
      split_router: false,
      auto_v2_lp: false,
      exemption_directory: false,
      variable_rates: false,
      launch_guards: false,
    })
    renderManage(MANAGER)
    expect(await screen.findByTestId('manage-buy-pct')).toBeDisabled()
    expect(screen.getByTestId('manage-sell-pct')).toBeDisabled()
    expect(screen.getByTestId('manage-buy-pct')).toHaveAttribute(
      'placeholder',
      expect.stringMatching(/Change rates later/)
    )
  })

  it('#610 AutoLP pair copy names this token CL8Y factory pair', async () => {
    const { queryCommunityTaxConfig, queryCommunityTaxFeatures } =
      await import('@/services/terraclassic/communityTaxToken')
    vi.mocked(queryCommunityTaxFeatures).mockResolvedValueOnce({
      mint_control: false,
      transfer_tax: false,
      split_router: false,
      auto_v2_lp: true,
      exemption_directory: false,
      variable_rates: true,
      launch_guards: false,
    })
    vi.mocked(queryCommunityTaxConfig).mockResolvedValueOnce({
      manager: MANAGER,
      treasury: MANAGER,
      buy_bps: 100,
      sell_bps: 100,
      transfer_bps: 0,
      max_buy_bps: 1000,
      max_sell_bps: 1000,
      max_transfer_bps: 500,
      factory: MANAGER,
      router: null,
      ust1: MANAGER,
      cmm_treasury: MANAGER,
      autolp: TOKEN,
      sinks: [],
      launch_guards: null,
      mint_revoked: false,
    })
    renderManage(MANAGER)
    expect(await screen.findByTestId('manage-autolp-pair-hint')).toHaveTextContent("this token's CL8Y factory pair")
    expect(screen.getByPlaceholderText("This token's CL8Y factory pair")).toBeInTheDocument()
  })

  it('P19: tax placeholders are percent not bps', async () => {
    renderManage(MANAGER)
    expect(await screen.findByTestId('manage-save-copy')).toBeInTheDocument()
    expect(await screen.findByTestId('manage-buy-pct')).toHaveAttribute('placeholder', '1.00')
    expect(screen.getByTestId('manage-sell-pct')).toHaveAttribute('placeholder', '1.00')
  })

  it('#633 hides register alert when there is no factory pair', async () => {
    renderManage(MANAGER)
    expect(await screen.findByTestId('manage-token-page')).toBeInTheDocument()
    expect(screen.queryByTestId('manage-register-alert')).not.toBeInTheDocument()
  })

  it('#633 manager sees one register button for the highest-LP unregistered pair', async () => {
    const { loadUnregisteredFactoryPairs, tokensNeedingRegisterForManager } =
      await import('@/utils/communityTaxRegisterPair')
    vi.mocked(loadUnregisteredFactoryPairs).mockResolvedValueOnce([
      {
        pair: 'terra1lowxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
        symbols: ['DEMO', 'EMBER'],
        usdTvl: 1,
        taxReserve: 1n,
        otherReserve: 1n,
      },
      {
        pair: 'terra1highxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
        symbols: ['DEMO', 'UST1'],
        usdTvl: 99,
        taxReserve: 1n,
        otherReserve: 1n,
      },
    ])
    vi.mocked(tokensNeedingRegisterForManager).mockResolvedValueOnce([
      { address: 'terra1othertokenxxxxxxxxxxxxxxxxxxxxxxxxxxx', symbol: 'OTH' },
    ])
    renderManage(MANAGER)
    expect(await screen.findByTestId('manage-register-alert')).toBeInTheDocument()
    expect(screen.getByTestId('manage-register-largest')).toHaveTextContent('DEMO/UST1')
    expect(screen.getByTestId('manage-register-largest')).toHaveTextContent(/largest pool/)
    expect(screen.getByTestId('manage-register-other')).toHaveTextContent('OTH')
  })

  it('#633 non-manager never sees the register CTA', async () => {
    const { loadUnregisteredFactoryPairs } = await import('@/utils/communityTaxRegisterPair')
    vi.mocked(loadUnregisteredFactoryPairs).mockResolvedValueOnce([
      {
        pair: 'terra1pairxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
        symbols: ['DEMO', 'UST1'],
        usdTvl: 1,
        taxReserve: 1n,
        otherReserve: 1n,
      },
    ])
    renderManage(OTHER)
    expect(await screen.findByTestId('manage-readonly')).toBeInTheDocument()
    expect(screen.queryByTestId('manage-register-alert')).not.toBeInTheDocument()
    expect(screen.queryByTestId('manage-register-largest')).not.toBeInTheDocument()
  })

  it('shows Unverified admin when wasm admin is not CMM', async () => {
    const { getChainContractInfo } = await import('@/services/terraclassic/queries')
    vi.mocked(getChainContractInfo).mockResolvedValueOnce({
      code_id: 11611,
      admin: OTHER,
      creator: MANAGER,
      label: 'community-tax-DEMO',
    })
    renderManage(MANAGER)
    expect(await screen.findByTestId('unverified-admin-banner')).toBeInTheDocument()
  })
})
