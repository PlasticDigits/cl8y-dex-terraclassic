import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import SwapPage from './SwapPage'
import { useWalletStore } from '@/hooks/useWallet'
import { FEE_DISCOUNT_REGISTRY_WARNING_TEXT } from '@/utils/feeDiscountRegistryWarning'

// GitLab #374: the registry-outage banner only renders when a fee-discount contract is
// configured. The default test env leaves VITE_FEE_DISCOUNT_ADDRESS empty, so this file
// stubs the constant truthy (isolated from SwapPage.test.tsx) to drive the page-level banner.
vi.mock('@/utils/constants', async (importActual) => {
  const actual = await importActual<typeof import('@/utils/constants')>()
  return {
    ...actual,
    FEE_DISCOUNT_CONTRACT_ADDRESS: 'terra1feediscount000000000000000000000000001',
  }
})

vi.mock('react-blockies', () => ({
  __esModule: true,
  default: function MockBlockies() {
    return null
  },
}))
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
  reverseSimulateSwap: vi.fn().mockResolvedValue({
    offer_amount: '1000000',
    spread_amount: '100',
    commission_amount: '3000',
  }),
  getPool: vi.fn().mockResolvedValue({
    assets: [
      { info: { token: { contract_addr: 'tokenA' } }, amount: '1000000' },
      { info: { token: { contract_addr: 'tokenB' } }, amount: '1000000' },
    ],
    total_share: '1000000',
  }),
}))
vi.mock('@/services/terraclassic/assetCodeIdFreeze', () => ({
  probePairCodeIdFreeze: vi.fn().mockResolvedValue({ frozen: false, verdict: 'tradable' }),
}))
vi.mock('@/services/terraclassic/settings', () => ({
  getPairFeeConfig: vi.fn().mockResolvedValue({ fee_bps: 30, treasury: '' }),
}))
vi.mock('@/services/terraclassic/feeDiscount', () => ({
  getTraderDiscount: vi.fn().mockResolvedValue({
    discount_bps: 250,
    needs_deregister: false,
    registration_epoch: 1,
  }),
  getRegistration: vi.fn().mockResolvedValue({ registered: true, tier_id: 1, tier: null }),
}))
vi.mock('@/services/terraclassic/pairDiscountRegistry', () => ({
  getPairDiscountRegistry: vi.fn().mockResolvedValue('terra1feediscount000000000000000000000000001'),
}))
vi.mock('@/services/terraclassic/swapRoutePreflight', () => ({
  preflightSwapRouteSpread: vi.fn().mockResolvedValue({
    worstSpreadPercent: '0.50',
    anyHopExceedsMaxSpread: false,
  }),
  enrichSwapOperationsWithHopMinReturns: vi.fn(async (operations: SwapOperation[]) => operations),
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

import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { findRoute, getAllTokens } from '@/services/terraclassic/router'
import type { SwapOperation } from '@/services/terraclassic/router'
import { simulateSwap } from '@/services/terraclassic/pair'
import { getRegistration, getTraderDiscount } from '@/services/terraclassic/feeDiscount'
import { getPairDiscountRegistry } from '@/services/terraclassic/pairDiscountRegistry'
import * as indexerClient from '@/services/indexer/client'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { getTokenBalance } from '@/services/terraclassic/queries'

const terraA = 'terra1from00000000000000000000000000000001'
const terraB = 'terra1to00000000000000000000000000000001'
const wallet = 'terra1wallet000000000000000000000000000001'

// Single-hop pair so a quote resolves and the Swap button can enable.
function seedSinglePair() {
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
}

describe('SwapPage fee-discount registry outage banner (GitLab #374)', () => {
  beforeEach(() => {
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
    seedSinglePair()
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')
    // Reset registration default each test — vitest mocks persist, so a prior reject would leak.
    vi.mocked(getRegistration).mockResolvedValue({ registered: true, tier_id: 1, tier: null })
    vi.mocked(getPairDiscountRegistry).mockResolvedValue('terra1feediscount000000000000000000000000001')
    // LCD pool sim succeeds; route/solve 400 falls back to LCD without the market-data outage banner (#326).
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('Indexer API error: 400 Bad Request'))
    vi.mocked(simulateSwap).mockResolvedValue({
      return_amount: '1000000',
      spread_amount: '100',
      commission_amount: '3000',
    })
  })

  it('shows the non-blocking warning when a registered trader hits indexer health ok:false, and Swap stays enabled', async () => {
    const user = userEvent.setup()
    vi.spyOn(indexerClient, 'getFeeDiscountHealth').mockResolvedValue({
      configured: true,
      fee_discount_registry_ok: false,
      consecutive_lcd_failures: 3,
    })

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '1')

    const banner = await screen.findByTestId('swap-fee-discount-registry-warning')
    expect(banner).toHaveTextContent(FEE_DISCOUNT_REGISTRY_WARNING_TEXT)
    // Non-blocking: submit remains enabled while the warning is shown.
    await waitFor(() => expect(screen.getByRole('button', { name: /^Swap$/i })).toBeEnabled())
  })

  it('shows the warning when the registration LCD query errors for a connected trader', async () => {
    const user = userEvent.setup()
    vi.spyOn(indexerClient, 'getFeeDiscountHealth').mockResolvedValue({
      configured: true,
      fee_discount_registry_ok: true,
      consecutive_lcd_failures: 0,
    })
    vi.mocked(getRegistration).mockRejectedValue(new Error('LCD registration query failed'))

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '1')

    expect(await screen.findByTestId('swap-fee-discount-registry-warning')).toHaveTextContent(
      FEE_DISCOUNT_REGISTRY_WARNING_TEXT
    )
  })

  it('does not show the warning when a registered trader has a healthy registry', async () => {
    const user = userEvent.setup()
    vi.spyOn(indexerClient, 'getFeeDiscountHealth').mockResolvedValue({
      configured: true,
      fee_discount_registry_ok: true,
      consecutive_lcd_failures: 0,
    })

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '1')

    await waitFor(() => expect(screen.getByRole('button', { name: /^Swap$/i })).toBeEnabled())
    expect(screen.queryByTestId('swap-fee-discount-registry-warning')).not.toBeInTheDocument()
  })

  it('does not show the warning for an unregistered trader on a healthy registry', async () => {
    const user = userEvent.setup()
    vi.spyOn(indexerClient, 'getFeeDiscountHealth').mockResolvedValue({
      configured: true,
      fee_discount_registry_ok: true,
      consecutive_lcd_failures: 0,
    })
    vi.mocked(getRegistration).mockResolvedValue({ registered: false, tier_id: null, tier: null })

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '1')

    await waitFor(() => expect(screen.getByRole('button', { name: /^Swap$/i })).toBeEnabled())
    expect(screen.queryByTestId('swap-fee-discount-registry-warning')).not.toBeInTheDocument()
  })

  it('strikethroughs the pair fee when the pair registry matches the configured contract (#537)', async () => {
    const user = userEvent.setup()
    vi.spyOn(indexerClient, 'getFeeDiscountHealth').mockResolvedValue({
      configured: true,
      fee_discount_registry_ok: true,
      consecutive_lcd_failures: 0,
    })

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '1')
    await waitFor(() => expect(screen.getByRole('button', { name: /^Swap$/i })).toBeEnabled(), { timeout: 5000 })
    await user.click(screen.getByText(/trade details/i))

    await waitFor(() => expect(document.querySelector('.line-through')).toBeTruthy())
    expect(screen.getByText('0.30%')).toBeInTheDocument()
  })

  it('shows full pair fee with no strikethrough when discount_registry is unset (#537)', async () => {
    const user = userEvent.setup()
    vi.mocked(getPairDiscountRegistry).mockResolvedValue(null)
    vi.mocked(getTraderDiscount).mockResolvedValue({
      discount_bps: 250,
      needs_deregister: false,
      registration_epoch: 1,
    })
    vi.spyOn(indexerClient, 'getFeeDiscountHealth').mockResolvedValue({
      configured: true,
      fee_discount_registry_ok: true,
      consecutive_lcd_failures: 0,
    })

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '1')
    await waitFor(() => expect(screen.getByRole('button', { name: /^Swap$/i })).toBeEnabled(), { timeout: 5000 })
    await user.click(screen.getByText(/trade details/i))

    await waitFor(() => expect(screen.getByText('0.30%')).toBeInTheDocument())
    expect(document.querySelector('.line-through')).toBeNull()
    expect(screen.queryByText(/Hold CL8Y/i)).not.toBeInTheDocument()
  })
})
