import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import SwapPage from './SwapPage'
import { useWalletStore } from '@/hooks/useWallet'

const { MOCK_LUNC_C } = vi.hoisted(() => ({
  MOCK_LUNC_C: 'terra1lunc_c_mock_address_for_testing_xxxxx',
}))

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
  simulateHybridSwap: vi.fn().mockResolvedValue({
    return_amount: '1000000',
    spread_amount: '100',
    commission_amount: '3000',
  }),
  swap: vi.fn().mockResolvedValue('txhash123'),
  getPairPaused: vi.fn().mockResolvedValue({ paused: false }),
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

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    FEE_DISCOUNT_CONTRACT_ADDRESS: 'terra1feediscount',
    WRAP_MAPPER_CONTRACT_ADDRESS: 'terra1wrap_mapper_mock',
    LUNC_C_TOKEN_ADDRESS: MOCK_LUNC_C,
    NATIVE_WRAPPED_PAIRS: {
      uluna: MOCK_LUNC_C,
    } as Record<string, string>,
    WRAPPED_NATIVE_PAIRS: {
      [MOCK_LUNC_C]: 'uluna',
    } as Record<string, string>,
  }
})

vi.mock('@/services/terraclassic/wrapMapper', () => ({
  queryPausedState: vi.fn().mockResolvedValue(false),
  checkRateLimitExceeded: vi.fn().mockResolvedValue(false),
}))

vi.mock('@/services/terraclassic/swapRoutePreflight', () => ({
  preflightSwapRouteSpread: vi.fn().mockResolvedValue({
    worstSpreadPercent: '0.50',
    anyHopExceedsMaxSpread: false,
  }),
  // Quote path calls this since #341 (76723cf); without it the mock throws and quotes die (GitLab #337).
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

vi.mock('@/hooks/useTradingBlacklist', () => ({
  useTradingBlacklist: vi.fn(),
}))

import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { findRoute, getAllTokens, isDirectWrapUnwrap, simulateMultiHopSwap } from '@/services/terraclassic/router'
import { queryPausedState, checkRateLimitExceeded } from '@/services/terraclassic/wrapMapper'
import type { SwapOperation } from '@/services/terraclassic/router'
import { simulateSwap, simulateHybridSwap, getPairPaused } from '@/services/terraclassic/pair'
import * as indexerClient from '@/services/indexer/client'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { getTokenBalance } from '@/services/terraclassic/queries'
import { getRegistration, getTraderDiscount } from '@/services/terraclassic/feeDiscount'
import { SIM_QUOTE_DEBOUNCE_MS } from '@/utils/quoteDebounce'
import { FEE_DISCOUNT_REGISTRY_WARNING_TEXT } from '@/utils/feeDiscountRegistryWarning'
import { spreadPercentFromRawSim } from '@/utils/rawAmountMath'
import { useTradingBlacklist } from '@/hooks/useTradingBlacklist'
import {
  TRADING_BLACKLIST_ALLOWED,
  pairBlacklistedResponse,
  tokenBlacklistedResponse,
  tradingBlacklistHookResult,
  walletBlacklistedResponse,
} from '@/test/tradingBlacklistMocks'
import { describeTradingBlacklistBlock } from '@/services/terraclassic/blacklist'
import { SWAP_SETTINGS_ADVANCED_OPEN_KEY } from '@/utils/swapSettingsAdvanced'

async function expandSwapAdvancedSettings(user: ReturnType<typeof userEvent.setup>) {
  const toggle = screen.getByTestId('swap-advanced-settings-toggle')
  if (toggle.getAttribute('aria-expanded') !== 'true') {
    await user.click(toggle)
  }
  await waitFor(() => expect(screen.getByRole('button', { name: /Compare indexer route/i })).toBeVisible())
}

async function openSwapSettingsWithAdvanced(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'Settings' }))
  await expandSwapAdvancedSettings(user)
}

describe('SwapPage', () => {
  beforeEach(() => {
    window.localStorage.removeItem(SWAP_SETTINGS_ADVANCED_OPEN_KEY)
    vi.mocked(useTradingBlacklist).mockReturnValue(TRADING_BLACKLIST_ALLOWED)
    vi.mocked(getAllPairsPaginated).mockResolvedValue({ pairs: [] })
    vi.mocked(findRoute).mockReturnValue(null)
    vi.mocked(getAllTokens).mockReturnValue([])
    vi.mocked(isDirectWrapUnwrap).mockReturnValue(null)
    vi.mocked(queryPausedState).mockResolvedValue(false)
    vi.mocked(checkRateLimitExceeded).mockResolvedValue(false)
    vi.mocked(getPairPaused).mockResolvedValue({ paused: false })
    vi.mocked(getConnectedWallet).mockReturnValue(null)
    useWalletStore.setState({ address: null, walletType: null, error: null })
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('indexer not used in this test'))
    vi.spyOn(indexerClient, 'getFeeDiscountHealth').mockResolvedValue({
      configured: true,
      fee_discount_registry_ok: true,
      consecutive_lcd_failures: 0,
    })
    vi.mocked(getRegistration).mockResolvedValue({ registered: false, tier_id: null, tier: null })
    vi.mocked(getTraderDiscount).mockResolvedValue({
      discount_bps: 0,
      needs_deregister: false,
      registration_epoch: null,
    })
  })

  it('renders without crashing', async () => {
    renderWithProviders(<SwapPage />)
    expect(await screen.findByRole('heading', { name: /^swap$/i })).toBeInTheDocument()
  })

  it('shows loading pairs state while factory pairs fetch is pending', async () => {
    let resolvePairs!: (value: { pairs: [] }) => void
    vi.mocked(getAllPairsPaginated).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolvePairs = resolve
        })
    )
    renderWithProviders(<SwapPage />)
    expect(screen.getByText(/loading pairs/i)).toBeInTheDocument()
    resolvePairs({ pairs: [] })
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
  })

  it('uses hybrid LCD quote when indexer POST fails and hybrid book leg is set (#418)', async () => {
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
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('indexer not used'))
    vi.spyOn(indexerClient, 'postRouteSolve').mockRejectedValue(new Error('indexer down'))
    vi.mocked(simulateHybridSwap).mockResolvedValue({
      return_amount: '4200000',
      spread_amount: '0',
      commission_amount: '0',
    })

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })

    await openSwapSettingsWithAdvanced(user)
    await user.type(screen.getByPlaceholderText('0.0'), '0.01')
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    await waitFor(() => expect(simulateHybridSwap).toHaveBeenCalled())
    expect(screen.queryByText(/pool-only simulation/i)).not.toBeInTheDocument()
  })

  describe('Swap Settings progressive disclosure (#413)', () => {
    const mockDirectCwPair = () => {
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
      return { terraA, terraB }
    }

    it('shows retail prefs only when Settings first opens', async () => {
      const user = userEvent.setup()
      mockDirectCwPair()
      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })

      await user.click(screen.getByRole('button', { name: 'Settings' }))

      const retailPanel = within(document.getElementById('swap-slippage-settings') as HTMLElement)
      expect(retailPanel.getByText('Slippage protection')).toBeInTheDocument()
      expect(retailPanel.getByText('Transaction deadline')).toBeInTheDocument()
      expect(retailPanel.getByTestId('swap-expert-mode-toggle')).toBeInTheDocument()
      expect(screen.getByTestId('swap-advanced-settings-toggle')).toHaveAttribute('aria-expanded', 'false')
      expect(screen.queryByTestId('swap-indexer-route-check')).not.toBeInTheDocument()
    })

    it('shows hybrid and indexer route check when Advanced is expanded', async () => {
      const user = userEvent.setup()
      mockDirectCwPair()
      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })

      await user.click(screen.getByRole('button', { name: 'Settings' }))
      await expandSwapAdvancedSettings(user)

      expect(screen.getByRole('checkbox', { name: /Route part of input through the limit book/i })).toBeInTheDocument()
      expect(screen.getByRole('button', { name: /Compare indexer route/i })).toBeInTheDocument()
    })

    it('persists Advanced expanded state in localStorage', async () => {
      const user = userEvent.setup()
      mockDirectCwPair()
      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })

      await user.click(screen.getByRole('button', { name: 'Settings' }))
      await expandSwapAdvancedSettings(user)

      expect(window.localStorage.getItem(SWAP_SETTINGS_ADVANCED_OPEN_KEY)).toBe('1')
    })
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
    vi.spyOn(indexerClient, 'getRouteSolve').mockReset()
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('indexer not used in this test'))
    vi.spyOn(indexerClient, 'postRouteSolve').mockResolvedValue({
      token_in: terraA,
      token_out: terraB,
      hops: [],
      router_operations: [],
      quote_kind: 'indexer_hybrid_lcd',
      estimated_amount_out: '5000',
    })

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })

    await openSwapSettingsWithAdvanced(user)
    expect(screen.getByRole('checkbox', { name: /Route part of input through the limit book/i })).toBeChecked()
    await user.type(screen.getByPlaceholderText('0.0'), '0.01')
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent(/Quote may change before submit/i)
    const docLink = within(alert).getByRole('link', { name: /^Docs$/i })
    expect(docLink.getAttribute('href')).toContain('limit-orders.md')

    const execution = await screen.findByTestId('swap-execution-summary')
    expect(execution).toHaveTextContent(/Execution:\s*Limit book \+ pool/i)
    expect(execution).toHaveTextContent(/Hybrid \(pool \+ limit book\)/i)
  })

  it('reconciles direct-hybrid receive to wallet when indexer estimate disagrees (#471)', async () => {
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
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('indexer not used in this test'))
    vi.spyOn(indexerClient, 'postRouteSolve').mockResolvedValue({
      token_in: terraA,
      token_out: terraB,
      hops: [{ pair: 'terra1pair', offer_token: terraA, ask_token: terraB }],
      router_operations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: terraA } },
            ask_asset_info: { token: { contract_addr: terraB } },
          },
        },
      ],
      quote_kind: 'indexer_hybrid_lcd',
      estimated_amount_out: '9999999',
      slippage_percent: '0.1',
      spot_amount_out: '10000000',
    })
    vi.mocked(simulateHybridSwap).mockResolvedValue({
      return_amount: '5000000',
      spread_amount: '0',
      commission_amount: '0',
    })

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })

    await openSwapSettingsWithAdvanced(user)
    await user.type(screen.getByPlaceholderText('0.0'), '0.01')
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    await waitFor(() => expect(simulateHybridSwap).toHaveBeenCalled())
    const reconciled = await screen.findByTestId('swap-direct-hybrid-amount-reconciled')
    expect(reconciled).toHaveTextContent(/Receive amount adjusted/i)
    expect(screen.queryByText(/9\.999/)).not.toBeInTheDocument()
    expect(screen.getAllByText('5.000').length).toBeGreaterThan(0)
  })

  it('shows market-data outage banner when sim fails with indexer transport error (GitLab #241)', async () => {
    const user = userEvent.setup()
    const wallet = 'terra1wallet000000000000000000000000000001'
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
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
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(simulateSwap).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    const banner = await screen.findByTestId('swap-market-data-outage-banner')
    expect(banner).toHaveTextContent(/market data service unavailable/i)
    expect(banner).not.toHaveTextContent(/VITE_INDEXER_URL|127\.0\.0\.1/i)
    expect(screen.getByTestId('swap-quote-unavailable')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Quote unavailable' })).toBeDisabled()
  })

  it('shows calm retry guidance when indexer returns HTTP 429 (SEC-E04 / GitLab #426)', async () => {
    const user = userEvent.setup()
    const wallet = 'terra1wallet000000000000000000000000000001'
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
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
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('Indexer API error: 429 Too Many Requests'))
    vi.mocked(simulateSwap).mockRejectedValue(new Error('Indexer API error: 429 Too Many Requests'))
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    const retry = await screen.findByTestId('swap-quote-retry-error')
    expect(retry).toHaveTextContent(/wait a moment and try again/i)
    expect(retry).not.toHaveTextContent(/\b429\b/)
    expect(retry).not.toHaveTextContent(/Indexer API error/i)
    expect(retry).not.toHaveTextContent(/127\.0\.0\.1|https?:\/\//i)
    expect(screen.queryByTestId('swap-market-data-outage-banner')).not.toBeInTheDocument()
    expect(screen.getByTestId('retry-error-button')).toBeInTheDocument()
  })

  it('shows outage banner with pool fallback quote when indexer fails but LCD sim succeeds (GitLab #241)', async () => {
    const user = userEvent.setup()
    const wallet = 'terra1wallet000000000000000000000000000001'
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
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
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(simulateSwap).mockResolvedValue({
      return_amount: '1000000',
      spread_amount: '100',
      commission_amount: '3000',
    })
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    expect(await screen.findByTestId('swap-market-data-outage-banner')).toBeInTheDocument()
    await waitFor(() => expect(screen.getByRole('button', { name: /^Swap$/i })).toBeEnabled())
  })

  it('does not show outage banner when route/solve returns 400 but LCD fallback succeeds (GitLab #326)', async () => {
    const user = userEvent.setup()
    const wallet = 'terra1wallet000000000000000000000000000001'
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
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
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('Indexer API error: 400 Bad Request'))
    vi.mocked(simulateSwap).mockResolvedValue({
      return_amount: '1000000',
      spread_amount: '100',
      commission_amount: '3000',
    })
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    await waitFor(() => expect(screen.getByRole('button', { name: /^Swap$/i })).toBeEnabled())
    expect(screen.queryByTestId('swap-market-data-outage-banner')).not.toBeInTheDocument()
  })

  it('shows client BFS fallback label when multihop submit uses client graph without indexer ops (#329)', async () => {
    const user = userEvent.setup()
    const wallet = 'terra1wallet000000000000000000000000000001'
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
    const terraA = 'terra1aa0000000000000000000000000000000001'
    const terraB = 'terra1bb0000000000000000000000000000000001'
    const terraC = 'terra1cc0000000000000000000000000000000001'
    const multihopRoute = [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraA } },
          ask_asset_info: { token: { contract_addr: terraB } },
        },
      },
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraB } },
          ask_asset_info: { token: { contract_addr: terraC } },
        },
      },
    ] as never
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          contract_addr: 'terra1pairab000000000000000000000000000001',
          liquidity_token: 'terra1lpab000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraA } }, { token: { contract_addr: terraB } }],
        },
        {
          contract_addr: 'terra1pairbc000000000000000000000000000001',
          liquidity_token: 'terra1lpbc000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraB } }, { token: { contract_addr: terraC } }],
        },
      ],
    })
    vi.mocked(getAllTokens).mockReturnValue([terraA, terraC, terraB])
    vi.mocked(findRoute).mockReturnValue(multihopRoute)
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(simulateMultiHopSwap).mockResolvedValue({ amount: '1000000' })
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    expect(await screen.findByTestId('swap-route-summary')).toBeInTheDocument()
    expect(await screen.findByTestId('swap-route-source-client-fallback')).toHaveTextContent(/client graph/i)
  })

  it('does not show client BFS fallback label when indexer multihop ops are used (#329)', async () => {
    const user = userEvent.setup()
    const wallet = 'terra1wallet000000000000000000000000000001'
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
    const terraA = 'terra1aa0000000000000000000000000000000001'
    const terraB = 'terra1bb0000000000000000000000000000000001'
    const terraC = 'terra1cc0000000000000000000000000000000001'
    const multihopRoute = [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraA } },
          ask_asset_info: { token: { contract_addr: terraB } },
        },
      },
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraB } },
          ask_asset_info: { token: { contract_addr: terraC } },
        },
      },
    ] as never
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          contract_addr: 'terra1pairab000000000000000000000000000001',
          liquidity_token: 'terra1lpab000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraA } }, { token: { contract_addr: terraB } }],
        },
        {
          contract_addr: 'terra1pairbc000000000000000000000000000001',
          liquidity_token: 'terra1lpbc000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraB } }, { token: { contract_addr: terraC } }],
        },
      ],
    })
    vi.mocked(getAllTokens).mockReturnValue([terraA, terraC, terraB])
    vi.mocked(findRoute).mockReturnValue(multihopRoute)
    vi.spyOn(indexerClient, 'getRouteSolve').mockResolvedValue({
      token_in: terraA,
      token_out: terraC,
      hops: [
        { offer_token: terraA, ask_token: terraB },
        { offer_token: terraB, ask_token: terraC },
      ],
      router_operations: multihopRoute,
      quote_kind: 'indexer_hybrid_lcd',
      estimated_amount_out: '1000000',
      intermediate_tokens: [terraA, terraB, terraC],
    })
    vi.mocked(simulateMultiHopSwap).mockResolvedValue({ amount: '1000000' })
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    await screen.findByTestId('swap-route-summary')
    expect(screen.queryByTestId('swap-route-source-client-fallback')).not.toBeInTheDocument()
  })

  it('reconciles tampered indexer intermediate_tokens to ops path and notifies user (#450 / SEC-I02 H09)', async () => {
    const user = userEvent.setup()
    const wallet = 'terra1wallet000000000000000000000000000001'
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
    const terraA = 'terra1aa0000000000000000000000000000000001'
    const terraB = 'terra1bb0000000000000000000000000000000001'
    const terraC = 'terra1cc0000000000000000000000000000000001'
    const terraEvil = 'terra1evil00000000000000000000000000000001'
    const multihopRoute = [
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraA } },
          ask_asset_info: { token: { contract_addr: terraEvil } },
        },
      },
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraEvil } },
          ask_asset_info: { token: { contract_addr: terraC } },
        },
      },
    ] as never
    vi.mocked(getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          contract_addr: 'terra1pairab000000000000000000000000000001',
          liquidity_token: 'terra1lpab000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraA } }, { token: { contract_addr: terraB } }],
        },
        {
          contract_addr: 'terra1pairbc000000000000000000000000000001',
          liquidity_token: 'terra1lpbc000000000000000000000000000001',
          asset_infos: [{ token: { contract_addr: terraB } }, { token: { contract_addr: terraC } }],
        },
      ],
    })
    vi.mocked(getAllTokens).mockReturnValue([terraA, terraC, terraB, terraEvil])
    vi.mocked(findRoute).mockReturnValue([
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraA } },
          ask_asset_info: { token: { contract_addr: terraB } },
        },
      },
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: terraB } },
          ask_asset_info: { token: { contract_addr: terraC } },
        },
      },
    ] as never)
    vi.spyOn(indexerClient, 'getRouteSolve').mockResolvedValue({
      token_in: terraA,
      token_out: terraC,
      hops: [
        { offer_token: terraA, ask_token: terraEvil },
        { offer_token: terraEvil, ask_token: terraC },
      ],
      router_operations: multihopRoute,
      quote_kind: 'indexer_hybrid_lcd',
      estimated_amount_out: '1000000',
      // Display claims A→B→C but ops route through evil token.
      intermediate_tokens: [terraA, terraB, terraC],
    })
    vi.mocked(simulateMultiHopSwap).mockResolvedValue({ amount: '1000000' })
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    const routeSummary = await screen.findByTestId('swap-route-summary')
    expect(routeSummary).toHaveTextContent(terraEvil)
    expect(routeSummary).not.toHaveTextContent(terraB)
    expect(await screen.findByTestId('swap-route-intermediate-reconciled')).toHaveTextContent(/route adjusted/i)
    expect(screen.queryByTestId('swap-route-source-client-fallback')).not.toBeInTheDocument()
  })

  it('rejects invalid characters in book leg amount without surfacing BigInt errors', async () => {
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

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })

    await openSwapSettingsWithAdvanced(user)
    const bookInput = screen.getByPlaceholderText('0.0')
    await user.type(bookInput, '1^2')
    expect(bookInput).toHaveValue('12')
    expect(screen.queryByText(/Cannot convert/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/Something went wrong/i)).not.toBeInTheDocument()
  })

  it('blocks swap above 30% route slippage unless Expert Mode is enabled (GitLab #293)', async () => {
    const user = userEvent.setup()
    const wallet = 'terra1wallet000000000000000000000000000001'
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
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
    vi.mocked(findRoute).mockReturnValue(null)
    vi.spyOn(indexerClient, 'getRouteSolve').mockResolvedValue({
      token_in: terraA,
      token_out: terraB,
      hops: [{ pair: 'terra1pair', offer_token: terraA, ask_token: terraB }],
      router_operations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: terraA } },
            ask_asset_info: { token: { contract_addr: terraB } },
          },
        },
      ],
      quote_kind: 'indexer_hybrid_lcd',
      estimated_amount_out: '36000000000',
      slippage_percent: '99.97',
      spot_amount_out: '960000',
    })
    vi.mocked(simulateMultiHopSwap).mockResolvedValue({ amount: '36000000000' })
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByPlaceholderText('0.00'), '1')

    // Wallet-aligned slippage (return vs spot) rounds to 100% for this arb-sized gap.
    expect(await screen.findByTestId('swap-expected-slippage')).toHaveTextContent('100.00%')
    expect(screen.getByTestId('swap-slippage-blocked')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Slippage is too high' })).toBeDisabled()
    expect(screen.getByTestId('swap-extreme-slippage-warning')).toBeInTheDocument()

    await user.click(screen.getByTestId('swap-enable-expert-mode'))
    const modal = await screen.findByRole('dialog')
    await user.type(screen.getByTestId('expert-mode-confirm-input'), 'ENABLE EXPERT MODE')
    await user.click(within(modal).getByTestId('expert-mode-confirm-enable'))

    await waitFor(() => {
      expect(screen.queryByTestId('swap-slippage-blocked')).not.toBeInTheDocument()
      expect(screen.queryByRole('button', { name: 'Slippage is too high' })).not.toBeInTheDocument()
    })
  })

  describe('large-amount precision (GitLab #366)', () => {
    const aboveSafeInt = '9007199254740992'

    it('gates positive human amounts above Number.MAX_SAFE_INTEGER without parseFloat loss', async () => {
      const user = userEvent.setup()
      const wallet = 'terra1wallet000000000000000000000000000001'
      vi.mocked(getConnectedWallet).mockReturnValue({} as never)
      useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
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
      vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('indexer not used in this test'))
      const spread = '891712726219358'
      const ret = '8917127262193583'
      vi.mocked(simulateSwap).mockResolvedValue({
        return_amount: ret,
        spread_amount: spread,
        commission_amount: '0',
      })
      vi.mocked(getTokenBalance).mockResolvedValue('999999999999999999999999')

      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })

      await user.type(screen.getByPlaceholderText('0.00'), aboveSafeInt)
      await waitFor(() => expect(simulateSwap).toHaveBeenCalled())

      expect(screen.queryByRole('button', { name: 'Enter Amount' })).not.toBeInTheDocument()
      const expectedPct = spreadPercentFromRawSim(ret, '0', spread)
      expect(await screen.findByTestId('swap-expected-slippage')).toHaveTextContent(`${expectedPct}%`)
    })
  })

  describe('submit–quote stale gate (GitLab #356)', () => {
    beforeEach(() => {
      vi.useFakeTimers({ shouldAdvanceTime: true })
    })

    afterEach(() => {
      vi.useRealTimers()
    })

    it('disables Swap with Calculating… while typed amount differs from debounced quote', async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
      const wallet = 'terra1wallet000000000000000000000000000001'
      vi.mocked(getConnectedWallet).mockReturnValue({} as never)
      useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
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
      vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('indexer not used in this test'))
      vi.mocked(simulateSwap).mockResolvedValue({
        return_amount: '1000000',
        spread_amount: '100',
        commission_amount: '3000',
      })
      vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })

      const payInput = screen.getByPlaceholderText('0.00')
      await user.type(payInput, '1')
      await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)
      await waitFor(() => expect(screen.getByRole('button', { name: /^Swap$/i })).toBeEnabled())

      await user.type(payInput, '0')

      expect(screen.getByRole('button', { name: /Calculating/i })).toBeDisabled()

      await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)
      await waitFor(() => expect(screen.getByRole('button', { name: /^Swap$/i })).toBeEnabled())
    })

    it('disables Swap with Calculating… while book leg differs from debounced hybrid quote (#360)', async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
      const wallet = 'terra1wallet000000000000000000000000000001'
      vi.mocked(getConnectedWallet).mockReturnValue({} as never)
      useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
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
      vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('indexer not used in this test'))
      vi.mocked(simulateSwap).mockResolvedValue({
        return_amount: '1000000',
        spread_amount: '100',
        commission_amount: '3000',
      })
      vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })

      await openSwapSettingsWithAdvanced(user)

      const payInput = screen.getByPlaceholderText('0.00')
      await user.type(payInput, '10')
      const bookInput = screen.getByPlaceholderText('0.0')
      await user.type(bookInput, '2')
      await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)
      await waitFor(() => expect(screen.getByRole('button', { name: /^Swap$/i })).toBeEnabled())

      await user.clear(bookInput)
      await user.type(bookInput, '5')

      expect(screen.getByRole('button', { name: /Calculating/i })).toBeDisabled()

      await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)
      await waitFor(() => expect(screen.getByRole('button', { name: /^Swap$/i })).toBeEnabled())
    })
  })

  describe('trading blacklist UX (GitLab #388 / SEC-A02)', () => {
    const wallet = 'terra1wallet000000000000000000000000000001'
    const terraA = 'terra1from00000000000000000000000000000001'
    const terraB = 'terra1to00000000000000000000000000000001'

    async function renderConnectedDirectSwap() {
      const user = userEvent.setup()
      vi.mocked(getConnectedWallet).mockReturnValue({} as never)
      useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
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
      vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
      await user.type(screen.getByPlaceholderText('0.00'), '1')
      return { user }
    }

    it.each([
      ['wallet', walletBlacklistedResponse()],
      ['pair', pairBlacklistedResponse()],
      ['token', tokenBlacklistedResponse(terraA)],
    ] as const)('shows %s blacklist alert copy and disables Swap CTA', async (_variant, resp) => {
      vi.mocked(useTradingBlacklist).mockReturnValue(tradingBlacklistHookResult(resp))

      await renderConnectedDirectSwap()

      const alert = await screen.findByRole('alert')
      expect(alert).toHaveTextContent(describeTradingBlacklistBlock(resp))
      expect(screen.getByRole('button', { name: 'Trading restricted' })).toBeDisabled()
    })
  })

  describe('fee-discount registry outage warning (GitLab #374)', () => {
    const wallet = 'terra1wallet000000000000000000000000000001'
    const terraA = 'terra1from00000000000000000000000000000001'
    const terraB = 'terra1to00000000000000000000000000000001'

    async function renderConnectedDirectSwap() {
      const user = userEvent.setup()
      vi.mocked(getConnectedWallet).mockReturnValue({} as never)
      useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
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
      vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })

      const payInput = screen.getByPlaceholderText('0.00')
      await user.type(payInput, '1')
      await waitFor(() => expect(screen.getByRole('button', { name: /^Swap$/i })).toBeEnabled(), {
        timeout: 5000,
      })
      return { user }
    }

    it('shows outage warning when registration LCD fails for a registered wallet and keeps swap enabled', async () => {
      vi.mocked(getRegistration).mockRejectedValue(new Error('LCD unavailable'))
      vi.mocked(getTraderDiscount).mockResolvedValue({
        discount_bps: 500,
        needs_deregister: false,
        registration_epoch: 1,
      })

      await renderConnectedDirectSwap()

      expect(await screen.findByTestId('swap-fee-discount-registry-warning')).toHaveTextContent(
        FEE_DISCOUNT_REGISTRY_WARNING_TEXT
      )
      expect(screen.getByRole('button', { name: /^Swap$/i })).toBeEnabled()
      expect(screen.queryByText(/Hold CL8Y/i)).not.toBeInTheDocument()
    })

    it('shows Hold CL8Y CTA for unregistered wallet with healthy LCD and hides outage banner', async () => {
      vi.mocked(getRegistration).mockResolvedValue({ registered: false, tier_id: null, tier: null })
      vi.spyOn(indexerClient, 'getFeeDiscountHealth').mockResolvedValue({
        configured: true,
        fee_discount_registry_ok: true,
        consecutive_lcd_failures: 0,
      })

      await renderConnectedDirectSwap()

      expect(await screen.findByText(/Hold CL8Y/i)).toBeInTheDocument()
      expect(screen.queryByTestId('swap-fee-discount-registry-warning')).not.toBeInTheDocument()
    })

    it('shows outage warning when indexer reports registry down even if LCD registration succeeded', async () => {
      vi.mocked(getRegistration).mockResolvedValue({
        registered: true,
        tier_id: 1,
        tier: { min_cl8y_balance: '0', discount_bps: 500, governance_only: false },
      })
      vi.spyOn(indexerClient, 'getFeeDiscountHealth').mockResolvedValue({
        configured: true,
        fee_discount_registry_ok: false,
        consecutive_lcd_failures: 2,
      })

      await renderConnectedDirectSwap()

      expect(await screen.findByTestId('swap-fee-discount-registry-warning')).toHaveTextContent(
        FEE_DISCOUNT_REGISTRY_WARNING_TEXT
      )
      expect(screen.queryByText(/Hold CL8Y/i)).not.toBeInTheDocument()
    })
  })

  describe('wrap pause and wrap rate limit CTA (SEC-A02 / GitLab #389)', () => {
    const wallet = 'terra1wallet000000000000000000000000000001'

    async function renderConnectedNativeWrapSwap() {
      const user = userEvent.setup()
      vi.mocked(getConnectedWallet).mockReturnValue({} as never)
      useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
      vi.mocked(isDirectWrapUnwrap).mockImplementation((from, to) => {
        if (from === 'uluna' && to === MOCK_LUNC_C) return 'wrap'
        return null
      })
      vi.mocked(getAllPairsPaginated).mockResolvedValue({
        pairs: [
          {
            contract_addr: 'terra1pair00000000000000000000000000000001',
            liquidity_token: 'terra1lp000000000000000000000000000000001',
            asset_infos: [{ token: { contract_addr: 'uluna' } }, { token: { contract_addr: MOCK_LUNC_C } }],
          },
        ],
      })
      vi.mocked(getAllTokens).mockReturnValue(['uluna', MOCK_LUNC_C])
      vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
      await user.type(screen.getByPlaceholderText('0.00'), '1')
      return { user }
    }

    it('shows disabled Wrapping is Temporarily Paused CTA when wrap mapper is paused', async () => {
      vi.mocked(queryPausedState).mockResolvedValue(true)
      await renderConnectedNativeWrapSwap()

      const btn = await screen.findByRole('button', { name: 'Wrapping is Temporarily Paused' })
      expect(btn).toBeDisabled()
    })

    it('shows disabled Rate Limit Exceeded CTA when wrap mapper rate limit is exceeded', async () => {
      vi.mocked(checkRateLimitExceeded).mockResolvedValue(true)
      await renderConnectedNativeWrapSwap()

      const btn = await screen.findByRole('button', { name: 'Rate Limit Exceeded' })
      expect(btn).toBeDisabled()
    })

    // GitLab #463 (SEC-I05 F-04): a disabled button alone isn't visible feedback —
    // surface an inline alert with retry guidance below the form too.
    it('shows an inline rate-limit alert with retry guidance, not just the disabled button', async () => {
      vi.mocked(checkRateLimitExceeded).mockResolvedValue(true)
      await renderConnectedNativeWrapSwap()

      const banner = await screen.findByTestId('swap-wrap-rate-limit-banner')
      expect(banner).toHaveTextContent(/Daily wrap limit reached/i)
      expect(banner).toHaveTextContent(/try again later, or reduce the amount/i)
    })
  })

  describe('pair pause disabled swap CTA (SEC-B05 / GitLab #395)', () => {
    const wallet = 'terra1wallet000000000000000000000000000001'
    const terraA = 'terra1from00000000000000000000000000000001'
    const terraB = 'terra1to00000000000000000000000000000001'

    async function renderConnectedDirectSwap() {
      const user = userEvent.setup()
      vi.mocked(getConnectedWallet).mockReturnValue({} as never)
      useWalletStore.setState({ address: wallet, walletType: 'simulated', error: null })
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
      vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
      await user.type(screen.getByPlaceholderText('0.00'), '1')
      return { user }
    }

    it('shows pause banner and disables swap submit when route pair is paused', async () => {
      vi.mocked(getPairPaused).mockResolvedValue({ paused: true })
      await renderConnectedDirectSwap()

      expect(await screen.findByTestId('swap-pair-paused-banner')).toHaveTextContent(
        /Pair is paused — swaps are blocked/i
      )
      expect(screen.getByRole('button', { name: 'Pair is paused' })).toBeDisabled()
    })
  })

  describe('swap pre-sign confirmation panel (GitLab #409 / SEC-D11)', () => {
    it('shows labeled action, pair, amounts, max spread, min return, and chain before wallet signing', async () => {
      const user = userEvent.setup()
      const wallet = 'terra1wallet000000000000000000000000000001'
      vi.mocked(getConnectedWallet).mockReturnValue({} as never)
      useWalletStore.setState({ address: wallet, walletType: 'keplr', error: null })
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
      vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('indexer not used in this test'))
      vi.mocked(simulateSwap).mockResolvedValue({
        return_amount: '1000000',
        spread_amount: '100',
        commission_amount: '3000',
      })
      vi.mocked(getTokenBalance).mockResolvedValue('10000000000')

      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
      await user.type(screen.getByPlaceholderText('0.00'), '1')

      expect(await screen.findByTestId('swap-pre-submit-summary')).toBeInTheDocument()
      expect(screen.getByTestId('swap-confirm-action')).toHaveTextContent('Swap')
      expect(screen.getByTestId('swap-confirm-pair')).toHaveTextContent('→')
      expect(screen.getByTestId('swap-confirm-offer')).toHaveTextContent('1')
      expect(screen.getByTestId('swap-confirm-receive')).toHaveTextContent('1')
      expect(screen.getByTestId('swap-confirm-max-spread')).toHaveTextContent('5%')
      expect(screen.getByTestId('swap-confirm-min-return')).toHaveTextContent('0.95')
      expect(screen.getByTestId('swap-confirm-chain')).toHaveTextContent('LocalTerra')
    })
  })
})
