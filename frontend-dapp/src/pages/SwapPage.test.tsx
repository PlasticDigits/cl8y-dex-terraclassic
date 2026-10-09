import { useWalletStore } from '@/hooks/useWallet'
import { renderWithProviders } from '@/test-utils'
import { LOCAL_FROM, LOCAL_TO, LOCAL_WALLET, localPair, localQuote, localTokens } from '@/test/localRouteFixtures'
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useLocation } from 'react-router-dom'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import SwapPage from './SwapPage'
vi.mock('@/services/terraclassic/routeTaxFunding', () => ({
  assertRouteTaxFunding: vi.fn().mockResolvedValue(undefined),
  queryRouterTaxFundingVersion: vi.fn().mockResolvedValue(undefined),
  ROUTE_TAX_FUNDING_UNAVAILABLE: 'Route tax funding could not be verified.',
}))

const { MOCK_LUNC_C } = vi.hoisted(() => ({
  MOCK_LUNC_C: 'terra1lunc_c_mock_address_for_testing_xxxxx',
  MOCK_USTC_C: 'terra1ustc_c_mock_address_for_testing_xxxxx',
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
  queryContract: vi.fn(async (_addr: string, msg: unknown) => {
    if (msg && typeof msg === 'object' && 'token_info' in msg) {
      return { name: 'Dummy', symbol: 'DUM', decimals: 6, total_supply: '0' }
    }
    return {}
  }),
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
  reverseSimulateSwap: vi.fn().mockResolvedValue({
    offer_amount: '1000000',
    spread_amount: '100',
    commission_amount: '3000',
  }),
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

vi.mock('@/services/terraclassic/pairDiscountRegistry', () => ({
  getPairDiscountRegistry: vi.fn().mockResolvedValue('terra1feediscount'),
}))

vi.mock('@/utils/constants', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/constants')>()
  return {
    ...actual,
    FEE_DISCOUNT_CONTRACT_ADDRESS: 'terra1feediscount',
    WRAP_MAPPER_CONTRACT_ADDRESS: 'terra1wrap_mapper_mock',
    TREASURY_CONTRACT_ADDRESS: 'terra1treasury',
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
  queryWrapMapperConfig: vi.fn().mockResolvedValue({
    governance: 'terra1gov',
    treasury: 'terra1treasury',
    paused: false,
    fee_wrap_bps: 100,
    fee_unwrap_bps: 100,
  }),
  wrapMapperFeeBps: (
    config: { fee_wrap_bps?: number; fee_unwrap_bps?: number; fee_bps?: number } | null,
    kind: 'wrap' | 'unwrap'
  ) => {
    if (!config) return null
    if (config.fee_wrap_bps != null && config.fee_unwrap_bps != null) {
      return kind === 'wrap' ? config.fee_wrap_bps : config.fee_unwrap_bps
    }
    return config.fee_bps ?? null
  },
  queryRateLimit: vi.fn().mockResolvedValue({
    config: { max_amount_per_window: '110000000000000', window_seconds: 86400 },
    current_window_start: null,
    amount_used: '0',
  }),
  getNativeForWrapped: (addr: string) => (addr === MOCK_LUNC_C ? 'uluna' : null),
  wrapTreasuryMatchesEnv: (config: { treasury: string }) => config.treasury === 'terra1treasury',
  wrapUnwrapFeeNote: (kind: 'wrap' | 'unwrap', feeBps: number | null | undefined) => {
    if (feeBps == null) return `${kind === 'wrap' ? 'Wrap' : 'Unwrap'} fee unavailable`
    return feeBps > 0
      ? `${kind === 'wrap' ? 'Wrap' : 'Unwrap'} (1.00% fee)`
      : `${kind === 'wrap' ? 'Wrap' : 'Unwrap'} (1:1)`
  },
  netAfterWrapMapperFee: (amount: bigint) => amount,
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
  netCw20AfterNativeWrap: vi.fn().mockResolvedValue(1n),
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

vi.mock('@/services/terraclassic/assetCodeIdFreeze', () => ({
  probePairCodeIdFreeze: vi.fn().mockResolvedValue({ frozen: false, verdict: 'tradable' }),
}))

import { useTradingBlacklist } from '@/hooks/useTradingBlacklist'
import { sounds } from '@/lib/sounds'
import * as indexerClient from '@/services/indexer/client'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import { getTraderDiscount } from '@/services/terraclassic/feeDiscount'
import { getPairPaused, simulateSwap } from '@/services/terraclassic/pair'
import { getPairDiscountRegistry } from '@/services/terraclassic/pairDiscountRegistry'
import { getTokenBalance } from '@/services/terraclassic/queries'
import type { SwapOperation } from '@/services/terraclassic/router'
import {
  findRoute,
  findRouteWithNativeSupport,
  getAllTokens,
  isDirectWrapUnwrap,
  simulateMultiHopSwap,
} from '@/services/terraclassic/router'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { queryWrapMapperConfig } from '@/services/terraclassic/wrapMapper'
import { useDexStore } from '@/stores/dex'
import { TRADING_BLACKLIST_ALLOWED } from '@/test/tradingBlacklistMocks'
import { NUMBER_DISPLAY_MODE_KEY, readNumberDisplayMode } from '@/utils/numberDisplayPreference'
import { DEFAULT_SLIPPAGE_TOLERANCE_PERCENT } from '@/utils/slippageProtectionCopy'
import { SWAP_SETTINGS_ADVANCED_OPEN_KEY } from '@/utils/swapSettingsAdvanced'

function SwapLocationProbe() {
  const { pathname, search } = useLocation()
  return <output data-testid="swap-location-probe">{pathname + search}</output>
}

async function expandSwapAdvancedSettings(user: ReturnType<typeof userEvent.setup>) {
  const toggle = screen.getByTestId('swap-advanced-settings-toggle')
  if (toggle.getAttribute('aria-expanded') !== 'true') {
    await user.click(toggle)
  }
  await waitFor(() => expect(screen.getByRole('button', { name: /Compare indexer route/i })).toBeVisible())
}

describe('SwapPage local executable quotes (#1452)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
    useDexStore.setState({ expertMode: false, slippageTolerance: 5 })
    useWalletStore.setState({ address: LOCAL_WALLET, walletType: 'simulated', error: null })
    vi.mocked(getConnectedWallet).mockReturnValue({ address: LOCAL_WALLET } as never)
    vi.mocked(getTokenBalance).mockResolvedValue('10000000000')
    vi.mocked(useTradingBlacklist).mockReturnValue(TRADING_BLACKLIST_ALLOWED)
    vi.mocked(getAllTokens).mockReturnValue([LOCAL_FROM, LOCAL_TO])
    vi.mocked(findRoute).mockReturnValue([
      {
        terra_swap: {
          offer_asset_info: { token: { contract_addr: LOCAL_FROM } },
          ask_asset_info: { token: { contract_addr: LOCAL_TO } },
        },
      },
    ])
    vi.mocked(isDirectWrapUnwrap).mockReturnValue(null)
    vi.mocked(findRouteWithNativeSupport).mockReturnValue(null)
    vi.spyOn(indexerClient, 'getAllPairsPaged').mockResolvedValue([localPair])
    vi.spyOn(indexerClient, 'getPair').mockResolvedValue(localPair)
    vi.spyOn(indexerClient, 'getTokens').mockResolvedValue(localTokens)
    vi.spyOn(indexerClient, 'getRouteSolve').mockImplementation(async (from, to, amount, options) =>
      localQuote(from, to, amount, options)
    )
    vi.spyOn(indexerClient, 'postRouteSolve').mockRejectedValue(new Error('unused'))
    vi.spyOn(indexerClient, 'getRouteSolveProgress').mockResolvedValue({
      stage: 'idle',
      done: 0,
      total: 0,
      label: '',
      cache_hit: true,
      updated_at_ms: Date.now(),
    })
  })
  afterEach(() => vi.restoreAllMocks())
  async function ready() {
    const rendered = renderWithProviders(<SwapPage />, { route: `/?from=${LOCAL_FROM}&to=${LOCAL_TO}` })
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(indexerClient.getRouteSolve).toHaveBeenCalled())
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeEnabled())
    return rendered
  }
  it('shows one useful local quote and sends actual spread without RPC simulation', async () => {
    await ready()
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('1')
    expect(indexerClient.getRouteSolve).toHaveBeenCalledWith(
      LOCAL_FROM,
      LOCAL_TO,
      '1000000',
      expect.objectContaining({ maxSpread: '0.05', trader: LOCAL_WALLET })
    )
    expect(simulateSwap).not.toHaveBeenCalled()
    expect(simulateMultiHopSwap).not.toHaveBeenCalled()
    expect(getTraderDiscount).not.toHaveBeenCalled()
    expect(getPairDiscountRegistry).not.toHaveBeenCalled()
    expect(queryWrapMapperConfig).not.toHaveBeenCalled()
    expect(getPairPaused).not.toHaveBeenCalled()
  })
  it('keeps confirmation amounts and positive floors in the submitted single-hop router snapshot', async () => {
    await ready()
    expect(screen.getByTestId('swap-confirm-offer')).toHaveTextContent('1')
    expect(screen.getByTestId('swap-confirm-min-return')).toHaveTextContent('0.95')
    await userEvent.click(screen.getByTestId('swap-submit'))
    const { executeMultiHopSwap } = await import('@/services/terraclassic/router')
    await waitFor(() => expect(executeMultiHopSwap).toHaveBeenCalled())
    expect(vi.mocked(executeMultiHopSwap).mock.calls[0]).toEqual([
      LOCAL_WALLET,
      LOCAL_FROM,
      '1000000',
      expect.arrayContaining([
        expect.objectContaining({ terra_swap: expect.objectContaining({ min_return: '950000' }) }),
      ]),
      '0.05',
      '950000',
      undefined,
      expect.any(Number),
      expect.objectContaining({ generation: 'fixture1' }),
    ])
  })
  it('blocks stale pay inputs until the matching local quote arrives', async () => {
    await ready()
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '10' } })
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
    await waitFor(() =>
      expect(indexerClient.getRouteSolve).toHaveBeenCalledWith(LOCAL_FROM, LOCAL_TO, '10000000', expect.anything())
    )
    await waitFor(() => expect(screen.getByTestId('swap-you-receive')).toHaveValue('10'))
  })
  it('retains the displayed baseline on a failed refresh while blocking submission', async () => {
    const { queryClient } = await ready()
    vi.mocked(indexerClient.getRouteSolve).mockRejectedValue(new Error('Indexer API error: 503'))
    await queryClient.invalidateQueries({ queryKey: ['simulation'] })
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeDisabled())
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('1')
    expect(simulateSwap).not.toHaveBeenCalled()
    expect(simulateMultiHopSwap).not.toHaveBeenCalled()
  })
  it('rejects unavailable executable metadata rather than trusting a positive amount', async () => {
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async () => {
      const q = localQuote()
      delete q.execution_metadata
      return q
    })
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(indexerClient.getRouteSolve).toHaveBeenCalled())
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
    expect(simulateSwap).not.toHaveBeenCalled()
  })
  it.each([40, 99])('preserves the %s%% route risk guard with Expert Mode', async (impact) => {
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) => {
      const q = localQuote(from, to, amount, options)
      q.spot_amount_out = ((BigInt(q.estimated_amount_out!) * 100n) / BigInt(100 - impact)).toString()
      return q
    })
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(indexerClient.getRouteSolve).toHaveBeenCalled())
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeDisabled())
    useDexStore.setState({ expertMode: true })
    if (impact === 99) expect(screen.getByTestId('swap-submit')).toBeDisabled()
    else await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeEnabled())
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
      expect(retailPanel.getByRole('group', { name: 'Number display' })).toBeInTheDocument()
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

      expect(
        screen.queryByRole('checkbox', { name: /Route part of input through the limit book/i })
      ).not.toBeInTheDocument()
      expect(screen.getByTestId('swap-hybrid-book-settings')).toBeInTheDocument()
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

    it('persists accessible number-display chips and restores Full on remount (#1259)', async () => {
      const user = userEvent.setup()
      mockDirectCwPair()
      const view = renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
      await user.click(screen.getByRole('button', { name: 'Settings' }))

      const group = screen.getByRole('group', { name: 'Number display' })
      const compact = within(group).getByRole('button', { name: 'Compact', pressed: true })
      const full = within(group).getByRole('button', { name: 'Full', pressed: false })
      expect(compact).toHaveClass('min-h-[44px]', 'min-w-[44px]')
      expect(full).toHaveClass('min-h-[44px]', 'min-w-[44px]')
      expect(screen.getByTestId('swap-number-display-preview')).toHaveTextContent('1.235M')

      vi.mocked(sounds.playButtonPress).mockClear()
      await user.click(full)
      expect(full).toHaveAttribute('aria-pressed', 'true')
      expect(compact).toHaveAttribute('aria-pressed', 'false')
      expect(screen.getByTestId('swap-number-display-preview')).toHaveTextContent('1,234,567')
      expect(readNumberDisplayMode()).toBe('full')
      expect(window.localStorage.getItem(NUMBER_DISPLAY_MODE_KEY)).toBe('full')
      expect(sounds.playButtonPress).toHaveBeenCalledTimes(1)

      view.unmount()
      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
      await user.click(screen.getByRole('button', { name: 'Settings' }))
      expect(screen.getByTestId('swap-number-display-full')).toHaveAttribute('aria-pressed', 'true')
      await user.click(screen.getByTestId('swap-number-display-compact'))
      expect(readNumberDisplayMode()).toBe('compact')
      expect(screen.getByTestId('swap-number-display-preview')).toHaveTextContent('1.235M')
    })

    it.each([
      ['1234.', '1,234'],
      ['1234.5678919', '1,234.567891'],
    ])('signing amount matches raw precision for draft %s (#1259)', async (draft, signingAmount) => {
      const user = userEvent.setup()
      mockDirectCwPair()
      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
      const input = screen.getByTestId('swap-you-pay-amount')
      await user.type(input, draft)
      await waitFor(() => expect(screen.getByTestId('swap-confirm-offer')).toHaveTextContent(signingAmount))
      expect(input).toHaveValue(draft)
    })

    it('changes display without rewriting inputs, route URL, or quote requests (#1259)', async () => {
      const user = userEvent.setup()
      const { terraA, terraB } = mockDirectCwPair()
      vi.mocked(simulateSwap).mockResolvedValue({
        return_amount: '1234567000000',
        spread_amount: '0',
        commission_amount: '0',
      })
      renderWithProviders(
        <>
          <SwapPage />
          <SwapLocationProbe />
        </>,
        { route: `/swap?pay=${terraA}&receive=${terraB}&amount=1234.567` }
      )
      await waitFor(() => expect(screen.getByTestId('swap-you-receive')).toHaveValue('1234.567'))
      await user.click(screen.getByRole('button', { name: 'Settings' }))

      const input = screen.getByTestId('swap-you-pay-amount')
      const location = screen.getByTestId('swap-location-probe').textContent
      const swapCalls = vi.mocked(simulateSwap).mock.calls.length
      const routeCalls = vi.mocked(indexerClient.getRouteSolve).mock.calls.length
      const postCalls = vi.mocked(indexerClient.postRouteSolve).mock.calls.length

      await user.click(screen.getByTestId('swap-number-display-full'))
      expect(screen.getByTestId('swap-number-display-preview')).toHaveTextContent('1,234,567')
      expect(input).toHaveValue('1234.567')
      expect(screen.getByTestId('swap-you-receive')).toHaveValue('1234.567')
      expect(screen.getByTestId('swap-location-probe').textContent).toBe(location)
      expect(simulateSwap).toHaveBeenCalledTimes(swapCalls)
      expect(indexerClient.getRouteSolve).toHaveBeenCalledTimes(routeCalls)
      expect(indexerClient.postRouteSolve).toHaveBeenCalledTimes(postCalls)

      await user.click(screen.getByTestId('swap-number-display-compact'))
      expect(input).toHaveValue('1234.567')
      expect(screen.getByTestId('swap-you-receive')).toHaveValue('1234.567')
      expect(screen.getByTestId('swap-location-probe').textContent).toBe(location)
      expect(simulateSwap).toHaveBeenCalledTimes(swapCalls)
      expect(indexerClient.getRouteSolve).toHaveBeenCalledTimes(routeCalls)
      expect(indexerClient.postRouteSolve).toHaveBeenCalledTimes(postCalls)
    })
  })

  describe('Slippage protection preset grouping (#528)', () => {
    beforeEach(() => {
      useDexStore.setState({ slippageTolerance: DEFAULT_SLIPPAGE_TOLERANCE_PERCENT })
    })

    afterEach(() => {
      useDexStore.setState({ slippageTolerance: DEFAULT_SLIPPAGE_TOLERANCE_PERCENT })
    })

    it('keeps Custom outside the chip group and defaults 5% active', async () => {
      const user = userEvent.setup()
      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
      await user.click(screen.getByRole('button', { name: 'Settings' }))

      const group = screen.getByTestId('swap-slippage-presets')
      expect(group).toHaveAttribute('role', 'group')
      expect(within(group).queryByTestId('swap-slippage-custom')).not.toBeInTheDocument()
      expect(screen.getByTestId('swap-slippage-custom')).toBeInTheDocument()
      expect(screen.getByTestId('swap-slippage-preset-5')).toHaveClass('tab-glass-active')
      expect(screen.getByTestId('swap-slippage-preset-0.5')).toHaveClass('tab-glass-inactive')
    })

    it('clicking a preset updates the store and plays press sound', async () => {
      const user = userEvent.setup()
      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
      await user.click(screen.getByRole('button', { name: 'Settings' }))

      await user.click(screen.getByTestId('swap-slippage-preset-0.5'))
      expect(useDexStore.getState().slippageTolerance).toBe(0.5)
      expect(sounds.playButtonPress).toHaveBeenCalled()
      expect(screen.getByTestId('swap-slippage-preset-0.5')).toHaveClass('tab-glass-active')
    })

    it('sanitizes Custom input; range error below 0.01; clamp/warn above 5 and 50', async () => {
      const user = userEvent.setup()
      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
      await user.click(screen.getByRole('button', { name: 'Settings' }))

      const custom = screen.getByRole('textbox', { name: /Custom slippage protection/i })
      await user.type(custom, '1e9')
      expect((custom as HTMLInputElement).value).toBe('19')
      expect(useDexStore.getState().slippageTolerance).toBe(19)
      expect(screen.getByText(/High slippage protection increases front-running risk/i)).toBeInTheDocument()

      await user.clear(custom)
      await user.type(custom, '0')
      expect(screen.getByText(/Must be between 0.01% and 50%/i)).toBeInTheDocument()
      expect(useDexStore.getState().slippageTolerance).not.toBe(0)

      await user.clear(custom)
      await user.type(custom, '99')
      expect(useDexStore.getState().slippageTolerance).toBe(50)
      expect(screen.getByText(/Must be between 0.01% and 50%/i)).toBeInTheDocument()
    })
  })
})
