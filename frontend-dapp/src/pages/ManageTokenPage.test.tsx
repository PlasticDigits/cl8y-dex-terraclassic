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
    COMMUNITY_TOKEN_LAUNCHER: 'terra1af9xm63mev4hnf4z0nmmcsnd9f4lpac2vs205rmaeg3kdqlqudhq894lyz',
    UST1_TOKEN_ADDRESS: 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
    CMM_GOVERNANCE_ADDR: 'terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2',
    isCommunityTaxEnabled: () => mockEnabled(),
  }
})

vi.mock('@/services/indexer/client', () => ({
  getTokens: vi.fn().mockResolvedValue([]),
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
    max_buy_bps: 2500,
    max_sell_bps: 2500,
    max_transfer_bps: 2500,
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
}))

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

  it('Minting SKU is not in the unlock list', async () => {
    renderManage(MANAGER)
    expect(await screen.findByTestId('manage-unlock-sku')).toBeInTheDocument()
    expect(screen.getByTestId('manage-unlock-sku').textContent).not.toMatch(/Minting/)
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
