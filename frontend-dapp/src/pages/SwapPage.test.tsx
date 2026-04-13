import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import SwapPage from './SwapPage'

vi.mock('react-blockies', () => ({
  __esModule: true,
  default: function MockBlockies() {
    return null
  },
}))
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { findRoute, getAllTokens } from '@/services/terraclassic/router'
import * as indexerClient from '@/services/indexer/client'

vi.mock('@/services/terraclassic/factory', () => ({
  getAllPairsPaginated: vi.fn().mockResolvedValue({ pairs: [] }),
}))

vi.mock('@/services/terraclassic/wallet', () => ({
  getConnectedWallet: vi.fn().mockReturnValue(null),
}))

vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn().mockResolvedValue({}),
  getTokenBalance: vi.fn().mockResolvedValue('0'),
}))

vi.mock('@/services/terraclassic/pair', () => ({
  simulateSwap: vi.fn().mockResolvedValue({
    return_amount: '1000000',
    spread_amount: '100',
    commission_amount: '3000',
  }),
  swap: vi.fn().mockResolvedValue('txhash123'),
  getPool: vi.fn().mockResolvedValue({
    assets: [
      { info: { token: { contract_addr: 'tokenA' } }, amount: '1000000' },
      { info: { token: { contract_addr: 'tokenB' } }, amount: '1000000' },
    ],
    total_share: '1000000',
  }),
}))

vi.mock('@/services/terraclassic/settings', () => ({
  getPairFeeConfig: vi.fn().mockResolvedValue({
    fee_bps: 30,
    treasury: '',
  }),
}))

vi.mock('@/services/terraclassic/feeDiscount', () => ({
  getTraderDiscount: vi.fn().mockResolvedValue({ discount_bps: 0 }),
  getRegistration: vi.fn().mockResolvedValue({ registered: false }),
}))

vi.mock('@/services/terraclassic/router', () => ({
  findRoute: vi.fn().mockReturnValue(null),
  getAllTokens: vi.fn().mockReturnValue([]),
  simulateMultiHopSwap: vi.fn().mockResolvedValue({ amount: '1000000' }),
  executeMultiHopSwap: vi.fn().mockResolvedValue('txhash123'),
  isDirectWrapUnwrap: vi.fn().mockReturnValue(null),
  findRouteWithNativeSupport: vi.fn().mockReturnValue(null),
  simulateNativeSwap: vi.fn().mockResolvedValue({ amount: '1' }),
  executeNativeSwap: vi.fn().mockResolvedValue('tx'),
}))

vi.mock('@/lib/sounds', () => ({
  sounds: {
    playButtonPress: vi.fn(),
    playHover: vi.fn(),
    playSuccess: vi.fn(),
    playError: vi.fn(),
  },
}))

describe('SwapPage', () => {
  beforeEach(() => {
    vi.mocked(getAllPairsPaginated).mockResolvedValue({ pairs: [] })
    vi.mocked(findRoute).mockReturnValue(null)
    vi.mocked(getAllTokens).mockReturnValue([])
  })

  it('renders without crashing', () => {
    renderWithProviders(<SwapPage />)
    expect(screen.getByText(/swap/i)).toBeTruthy()
  })

  it('shows loading tokens state when no pairs loaded', () => {
    renderWithProviders(<SwapPage />)
    const loadingElements = screen.queryAllByText(/loading tokens/i)
    expect(loadingElements.length).toBeGreaterThan(0)
  })

  it('shows hybrid book warning with doc link before swap when book leg > 0', async () => {
    const user = userEvent.setup()
    const terraA = 'terra1from00000000000000000000000000000001'
    const terraB = 'terra1to00000000000000000000000000000001'
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          contract_addr: 'terra1pair00000000000000000000000000000001',
          liquidity_token: 'terra1lp000000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraA } }, { token: { contract_addr: terraB } }],
        },
      ],
    })
    vi.mocked(getAllTokens).mockReturnValue([terraA, terraB])
    vi.mocked(findRoute).mockReturnValue([
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraA } },
          ask_asset_info: { token: { contract_addr: terraB } },
        },
      },
    ] as never)
    vi.spyOn(indexerClient, 'postRouteSolve').mockResolvedValue({
      token_in: terraA,
      token_out: terraB,
      hops: [],
      router_operations: [],
      estimated_amount_out: '5000',
    })

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading tokens/i)).not.toBeInTheDocument(), { timeout: 5000 })

    await user.click(screen.getByRole('button', { name: 'Settings' }))
    await user.click(screen.getByRole('checkbox', { name: /Route part of input through the limit book/i }))
    await user.type(screen.getByPlaceholderText('0.0'), '0.01')
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent(/integrator semantics/i)
    const docLink = within(alert).getByRole('link', { name: /docs\/limit-orders\.md/i })
    expect(docLink.getAttribute('href')).toContain('docs/limit-orders.md')
  })
})
