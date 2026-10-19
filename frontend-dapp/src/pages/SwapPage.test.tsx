import { useWalletStore } from '@/hooks/useWallet'
import { renderWithProviders } from '@/test-utils'
import {
  LOCAL_FROM,
  LOCAL_TO,
  LOCAL_WALLET,
  LOCAL_PAIR,
  localPair,
  localQuote as rawLocalQuote,
  localTaxQuote,
  localTokens,
} from '@/test/localRouteFixtures'
import { act, fireEvent, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useLocation } from 'react-router-dom'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import SwapPage from './SwapPage'
import { attachZeroMovementRiskFixture } from '@/test/routeEconomicFixtures'

function localQuote(...args: Parameters<typeof rawLocalQuote>) {
  return attachZeroMovementRiskFixture(rawLocalQuote(...args))
}
const { mutationEntry } = vi.hoisted(() => ({
  mutationEntry: { run: undefined as undefined | (() => Promise<unknown>), toastOnError: true },
}))
vi.mock('@/hooks/useTerraBroadcastMutation', async (importActual) => {
  const actual = await importActual<typeof import('@/hooks/useTerraBroadcastMutation')>()
  return {
    ...actual,
    useTerraBroadcastMutation: (options: Parameters<typeof actual.useTerraBroadcastMutation>[0]) => {
      if (options.toastSuccess === 'Swap submitted.') {
        mutationEntry.run = () => options.mutationFn(undefined)
        mutationEntry.toastOnError = options.toastOnError !== false
      }
      return actual.useTerraBroadcastMutation(options)
    },
  }
})
vi.mock('@/services/terraclassic/routeTaxFunding', () => ({
  assertRouteTaxFunding: vi.fn().mockResolvedValue(undefined),
  queryRouterTaxFundingVersion: vi.fn().mockResolvedValue(undefined),
  ROUTE_TAX_FUNDING_UNAVAILABLE: 'Route tax funding could not be verified.',
}))

const { MOCK_LUNC_C } = vi.hoisted(() => ({
  MOCK_LUNC_C: 'terra1lunc_c_mock_address_for_testing_xxxxx',
  MOCK_USTC_C: 'terra1ustc_c_mock_address_for_testing_xxxxx',
}))

// These execution fixtures deliberately select their synthetic pair; product
// default identity coverage lives in SwapPage.queryParams/pairCatalogRank.
vi.mock('@/utils/pairCatalogRank', async (importActual) => ({
  ...(await importActual<typeof import('@/utils/pairCatalogRank')>()),
  defaultRetailSwapTokenPair: () => [LOCAL_FROM, LOCAL_TO] as [string, string],
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
  executeLocalNativeRoute: vi.fn().mockResolvedValue('tx'),
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
import { IndexerApiError } from '@/utils/routeQuoteErrors'
import * as swapFee from '@/services/terraclassic/swapNetworkFee'

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
  it('uses one inline error surface while retaining success notifications', async () => {
    renderWithProviders(<SwapPage />)
    expect(mutationEntry.toastOnError).toBe(false)
  })
  async function ready(route = `/?from=${LOCAL_FROM}&to=${LOCAL_TO}`) {
    const rendered = renderWithProviders(<SwapPage />, { route })
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(indexerClient.getRouteSolve).toHaveBeenCalled())
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeEnabled())
    return rendered
  }
  it.each([false, true])(
    'mutation entry rejects a queued %s native-pay submission after balance loss (#1480)',
    async (nativePay) => {
      if (nativePay) configureNativeTwoHopQuote()
      const { queryClient } = await ready(nativePay ? `/?from=uluna&to=${LOCAL_TO}` : undefined)
      expect(screen.getByTestId('swap-network-fee')).toHaveTextContent(nativePay ? '~181.3 LUNC' : '~90.64 LUNC')
      // Both fixtures still own enough principal. Only the reviewed fee budget
      // (plus native principal) is short, by one raw uluna.
      const required = nativePay ? 182_280_000n : 90_640_000n
      await act(async () => {
        queryClient.setQueryData(['tokenBalance', LOCAL_WALLET, 'uluna'], (required - 1n).toString())
      })
      await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeDisabled())
      // Invoke the actual production mutation entry, bypassing the button to
      // model a queued submit; the real hook still renders the component normally.
      await expect(mutationEntry.run!()).rejects.toThrow(/Need .* LUNC for network fee/)
      const { executeMultiHopSwap, executeLocalNativeRoute } = await import('@/services/terraclassic/router')
      expect(executeMultiHopSwap).not.toHaveBeenCalled()
      expect(executeLocalNativeRoute).not.toHaveBeenCalled()
      expect(queryWrapMapperConfig).not.toHaveBeenCalled()
      expect(simulateSwap).not.toHaveBeenCalled()
    }
  )

  it('mutation entry rejects unavailable fee coverage before execution preparation (#1480)', async () => {
    const blocker = 'Network fee unavailable for this route. Refresh the quote.'
    vi.spyOn(swapFee, 'tryEstimateSwapNetworkFee').mockReturnValue({
      gasLimit: 0,
      feeUluna: 0n,
      gasPriceUluna: 28.325,
      coverageError: blocker,
    })
    renderWithProviders(<SwapPage />, { route: `/?from=${LOCAL_FROM}&to=${LOCAL_TO}` })
    fireEvent.change(await screen.findByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(screen.getByTestId('swap-you-receive')).toHaveValue('1'))
    expect(screen.getByTestId('swap-submit')).toHaveTextContent('Network fee unavailable')
    await expect(mutationEntry.run!()).rejects.toThrow(blocker)
    const { executeMultiHopSwap, executeLocalNativeRoute } = await import('@/services/terraclassic/router')
    expect(executeMultiHopSwap).not.toHaveBeenCalled()
    expect(executeLocalNativeRoute).not.toHaveBeenCalled()
  })
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
  it.each([undefined, 1] as const)('actual Swap fee row follows reviewed protocol %s (#1480)', async (version) => {
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) =>
      version === 1
        ? attachZeroMovementRiskFixture(localTaxQuote(from, to, amount, options, 0, 0, amount))
        : localQuote(from, to, amount, options)
    )
    await ready()
    expect(screen.getByTestId('swap-network-fee')).toHaveTextContent(version === 1 ? '~73.64 LUNC' : '~90.64 LUNC')
    await userEvent.click(screen.getByTestId('swap-submit'))
    const { executeMultiHopSwap } = await import('@/services/terraclassic/router')
    await waitFor(() => expect(executeMultiHopSwap).toHaveBeenCalledTimes(1))
    expect(vi.mocked(executeMultiHopSwap).mock.calls[0][8]?.tax_funding_version).toBe(version)
    expect(simulateSwap).not.toHaveBeenCalled()
    expect(simulateMultiHopSwap).not.toHaveBeenCalled()
  })

  it('reference policy exclusion is a settled finite blocker, without raw remote prose (#1479)', async () => {
    vi.mocked(indexerClient.getRouteSolve).mockRejectedValue(new IndexerApiError(422, 'reference_policy_excluded'))
    renderWithProviders(<SwapPage />, { route: `/?from=${LOCAL_FROM}&to=${LOCAL_TO}` })
    fireEvent.change(await screen.findByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeDisabled())
    await screen.findByText('Reliable reference pricing is unavailable for this market.')
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('')
    expect(simulateSwap).not.toHaveBeenCalled()
  })
  function configureNativeTwoHopQuote() {
    vi.mocked(getTokenBalance).mockResolvedValue('1000000000')
    vi.mocked(getAllTokens).mockReturnValue(['uluna', MOCK_LUNC_C, LOCAL_FROM, LOCAL_TO])
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (_from, _to, amount, options) => {
      const intermediate = _to === LOCAL_FROM ? LOCAL_TO : LOCAL_FROM
      const first = localQuote(MOCK_LUNC_C, intermediate, amount, options)
      const second = localQuote(intermediate, _to, amount, options)
      const quote = {
        ...second,
        token_in: 'uluna',
        hops: [...first.hops, ...second.hops],
        router_operations: [...first.router_operations!, ...second.router_operations!],
        intermediate_tokens: [MOCK_LUNC_C, intermediate, _to],
        execution_metadata: {
          ...second.execution_metadata!,
          hops: [...first.execution_metadata!.hops, ...second.execution_metadata!.hops],
          wrapper: {
            input_token: 'uluna',
            output_token: _to,
            input_amount: amount,
            route_input_amount: amount,
            route_output_amount: amount,
            minimum_receive_base: second.execution_metadata!.minimum_receive,
            wrap_input: true,
            unwrap_output: false,
            treasury: 'terra1treasury',
            mapper: 'terra1wrap_mapper_mock',
          },
        },
      }
      return attachZeroMovementRiskFixture(quote)
    })
    const first = localQuote(MOCK_LUNC_C, LOCAL_FROM)
    const second = localQuote(LOCAL_FROM, LOCAL_TO)
    vi.mocked(findRouteWithNativeSupport).mockReturnValue({
      operations: [...first.router_operations!, ...second.router_operations!],
      needsWrapInput: true,
      needsUnwrapOutput: false,
    })
  }
  it('native progress watches original gross identity, pool mode and selected protection (#1489)', async () => {
    configureNativeTwoHopQuote()
    useDexStore.setState({ slippageTolerance: 3 })
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${LOCAL_TO}` })
    fireEvent.change(await screen.findByTestId('swap-you-pay-amount'), { target: { value: '10000' } })
    await waitFor(() =>
      expect(indexerClient.getRouteSolveProgress).toHaveBeenCalledWith(
        'uluna',
        expect.any(String),
        '10000000000',
        expect.objectContaining({ poolOnly: true, maxSpread: '0.03', maxMakerFills: 8 })
      )
    )
    const solve = vi.mocked(indexerClient.getRouteSolve).mock.calls.at(-1)!
    expect(indexerClient.getRouteSolveProgress).toHaveBeenCalledWith(
      solve[0],
      solve[1],
      solve[2],
      expect.objectContaining({ poolOnly: true, maxSpread: '0.03' })
    )
    expect(simulateSwap).not.toHaveBeenCalled()
    expect(simulateMultiHopSwap).not.toHaveBeenCalled()
  })
  it('adopts a higher same-cohort publication by refetching validated operations (#1489)', async () => {
    const { queryClient } = await ready()
    vi.mocked(indexerClient.getRouteSolveProgress).mockResolvedValue({
      stage: 'done',
      done: 1,
      total: 1,
      label: 'Local route ready',
      cache_hit: true,
      updated_at_ms: Date.now(),
      quote_generation: 'fixture1',
      best_amount_out: '1100000',
      search_complete: true,
    })
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) =>
      localQuote(from, to, amount, options, '1100000')
    )
    await queryClient.invalidateQueries({ queryKey: ['routeQuotePromotion'] })
    await waitFor(() => expect(screen.getByTestId('swap-you-receive')).toHaveValue('1.1'))
    expect(screen.getByTestId('swap-confirm-min-return')).toHaveTextContent('1.045')
    expect(simulateSwap).not.toHaveBeenCalled()
    expect(simulateMultiHopSwap).not.toHaveBeenCalled()
  })

  it('loads the native entry with an empty amount without inferring unavailable wrapping (#1489)', async () => {
    configureNativeTwoHopQuote()
    const catalog = await import('@/utils/pairCatalogRank')
    vi.spyOn(catalog, 'defaultRetailSwapTokenPair').mockReturnValue(['uluna', LOCAL_TO])
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
    expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('')
    expect(screen.queryByText('Wrapping is temporarily unavailable.')).not.toBeInTheDocument()
    expect(screen.getByTestId('swap-submit')).toHaveTextContent('Enter Amount')
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
    expect(indexerClient.getRouteSolve).not.toHaveBeenCalled()
    expect(queryWrapMapperConfig).not.toHaveBeenCalled()
    await expect(mutationEntry.run!()).rejects.toThrow('Missing parameters')
  })
  it.each(['', '0'])('returns to neutral native entry after clearing to %s (#1489)', async (empty) => {
    configureNativeTwoHopQuote()
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${LOCAL_TO}` })
    fireEvent.change(await screen.findByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeEnabled())
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: empty } })
    expect(screen.queryByText('Wrapping is temporarily unavailable.')).not.toBeInTheDocument()
    expect(screen.getByTestId('swap-submit')).toHaveTextContent('Enter Amount')
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
    await expect(mutationEntry.run!()).rejects.toThrow()
    const { executeLocalNativeRoute } = await import('@/services/terraclassic/router')
    expect(executeLocalNativeRoute).not.toHaveBeenCalled()
  })
  it('waits through native pay debounce/initial quote without an unavailable claim, then uses validated context (#1489)', async () => {
    configureNativeTwoHopQuote()
    const nativeSolve = vi.mocked(indexerClient.getRouteSolve).getMockImplementation()!
    let finish: (() => void) | undefined
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (...args) => {
      await new Promise<void>((resolve) => {
        finish = resolve
      })
      return nativeSolve(...args)
    })
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${LOCAL_TO}` })
    fireEvent.change(await screen.findByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    expect(screen.queryByText('Wrapping is temporarily unavailable.')).not.toBeInTheDocument()
    await waitFor(() => expect(finish).toBeDefined())
    expect(screen.getByTestId('swap-submit')).toHaveTextContent('Calculating')
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
    expect(screen.queryByText('Wrapping is temporarily unavailable.')).not.toBeInTheDocument()
    await expect(mutationEntry.run!()).rejects.toThrow('Quote unavailable')
    await act(async () => {
      finish!()
    })
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeEnabled())
    expect(screen.getByTestId('swap-you-receive')).not.toHaveValue('')
    expect(queryWrapMapperConfig).not.toHaveBeenCalled()
  })
  it('keeps missing native observations in a truthful pricing wait (#1489)', async () => {
    configureNativeTwoHopQuote()
    vi.mocked(indexerClient.getRouteSolve).mockRejectedValue(new IndexerApiError(503, 'missing_observations'))
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${LOCAL_TO}` })
    fireEvent.change(await screen.findByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(screen.getByTestId('swap-price-data-loading')).toBeInTheDocument())
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toHaveTextContent('Wait for price'))
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
    expect(screen.queryByText('Wrapping is temporarily unavailable.')).not.toBeInTheDocument()
    await expect(mutationEntry.run!()).rejects.toThrow()
  })
  it('does not turn impossible native pay dust into an endless loading state (#1489)', async () => {
    configureNativeTwoHopQuote()
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${LOCAL_TO}` })
    fireEvent.change(await screen.findByTestId('swap-you-pay-amount'), { target: { value: '0.0000001' } })
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toHaveTextContent('Quote unavailable'))
    expect(screen.queryByText('Wrapping is temporarily unavailable.')).not.toBeInTheDocument()
    expect(indexerClient.getRouteSolve).not.toHaveBeenCalled()
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
  })
  it('retains real positive native unsupported and treasury-mismatch refusals (#1489)', async () => {
    configureNativeTwoHopQuote()
    vi.mocked(indexerClient.getRouteSolve).mockRejectedValue(new IndexerApiError(422, 'unsupported_semantics'))
    const { queryClient } = renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${LOCAL_TO}` })
    fireEvent.change(await screen.findByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(screen.getByTestId('swap-quote-error')).toBeInTheDocument())
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
    await expect(mutationEntry.run!()).rejects.toThrow('Quote unavailable')
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(() => new Promise(() => {}))
    await act(async () => {
      queryClient.setQueryData(['wrapMapperConfig'], {
        treasury: 'terra1incorrect',
        paused: false,
        fee_wrap_bps: 100,
        fee_unwrap_bps: 100,
      })
      void queryClient.invalidateQueries({ queryKey: ['simulation'] })
    })
    expect(screen.getByTestId('swap-submit')).toHaveTextContent('Wrap treasury misconfigured')
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
    await expect(mutationEntry.run!()).rejects.toThrow('Quote unavailable')
    const { executeLocalNativeRoute } = await import('@/services/terraclassic/router')
    expect(executeLocalNativeRoute).not.toHaveBeenCalled()
  })
  it('handles native receive-target pending and below-precision intent without false wrapping status (#1489)', async () => {
    configureNativeTwoHopQuote()
    const nativeSolve = vi.mocked(indexerClient.getRouteSolve).getMockImplementation()!
    let finish: (() => void) | undefined
    vi.spyOn(indexerClient, 'getRouteSolveExactOutput').mockImplementation(async (from, to, target, options) => {
      await new Promise<void>((resolve) => {
        finish = resolve
      })
      const q = await nativeSolve(from, to, target, options)
      return { ...q, sizing_mode: 'local_pool_exact_output', required_amount_in: target, target_amount_out: target }
    })
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${LOCAL_TO}` })
    fireEvent.change(await screen.findByTestId('swap-you-receive'), { target: { value: '1' } })
    expect(screen.queryByText('Wrapping is temporarily unavailable.')).not.toBeInTheDocument()
    await waitFor(() => expect(finish).toBeDefined())
    expect(screen.getByTestId('swap-submit')).toHaveTextContent('Calculating')
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
    await act(async () => {
      finish!()
    })
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeEnabled())
    fireEvent.change(screen.getByTestId('swap-you-receive'), { target: { value: '0.0000001' } })
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toHaveTextContent('Increase receive amount'))
    expect(screen.getByTestId('swap-receive-amount-error')).toBeInTheDocument()
    expect(screen.queryByText('Wrapping is temporarily unavailable.')).not.toBeInTheDocument()
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
  })

  it('treats an expired native wrapper quote as stale pricing, not global unavailability (#1489)', async () => {
    configureNativeTwoHopQuote()
    const { queryClient } = renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${LOCAL_TO}` })
    fireEvent.change(await screen.findByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeEnabled())
    const count = vi.mocked(indexerClient.getRouteSolve).mock.calls.length
    const future = Date.now() + 20_000
    vi.spyOn(Date, 'now').mockReturnValue(future)
    await act(async () => {
      queryClient.setQueriesData({ queryKey: ['simulation'] }, (old: unknown) => old && { ...(old as object) })
    })
    await waitFor(() => expect(screen.getByTestId('swap-you-receive')).toHaveValue(''))
    expect(indexerClient.getRouteSolve).toHaveBeenCalledTimes(count)
    expect(screen.queryByText('Wrapping is temporarily unavailable.')).not.toBeInTheDocument()
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
    await expect(mutationEntry.run!()).rejects.toThrow('Quote unavailable')
  })
  it('retains a real cached wrapper pause at a positive amount (#1489)', async () => {
    configureNativeTwoHopQuote()
    const { queryClient } = renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${LOCAL_TO}` })
    await screen.findByTestId('swap-you-pay-amount')
    await act(async () => {
      queryClient.setQueryData(['wrapMapperConfig'], {
        treasury: 'terra1treasury',
        paused: true,
        fee_wrap_bps: 100,
        fee_unwrap_bps: 100,
      })
    })
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toHaveTextContent('Wrapping is Temporarily Paused'))
    expect(screen.getByTestId('swap-pair-paused-banner')).toHaveTextContent('Wrapping is paused')
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
    await expect(mutationEntry.run!()).rejects.toThrow('Trading is paused')
    const { executeLocalNativeRoute } = await import('@/services/terraclassic/router')
    expect(executeLocalNativeRoute).not.toHaveBeenCalled()
  })
  it('rejects oversized native input without starting an indefinite quote (#1489)', async () => {
    configureNativeTwoHopQuote()
    vi.mocked(indexerClient.getRouteSolve).mockRejectedValue(new IndexerApiError(422, 'invalid_request'))
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${LOCAL_TO}` })
    fireEvent.change(await screen.findByTestId('swap-you-pay-amount'), {
      target: { value: '340282366920938463463374607431768211456' },
    })
    expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('')
    expect(screen.getByTestId('swap-submit')).toHaveTextContent('Enter Amount')
    expect(indexerClient.getRouteSolve).not.toHaveBeenCalled()
    expect(screen.getByTestId('swap-submit')).not.toHaveTextContent('Calculating')
    expect(screen.queryByText('Wrapping is temporarily unavailable.')).not.toBeInTheDocument()
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
    await expect(mutationEntry.run!()).rejects.toThrow()
  })
  it('native wrap+two-hop fee and Max use the reviewed offline route (#1264 #1480)', async () => {
    configureNativeTwoHopQuote()
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${LOCAL_TO}` })
    fireEvent.change(await screen.findByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeEnabled())
    expect(screen.getByTestId('swap-network-fee')).toHaveTextContent('~181.3 LUNC')
    await userEvent.click(screen.getByTestId('swap-pay-max'))
    expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('818.72')
    await waitFor(() =>
      expect(indexerClient.getRouteSolve).toHaveBeenCalledWith(
        'uluna',
        expect.any(String),
        '818720000',
        expect.anything()
      )
    )
    expect(simulateSwap).not.toHaveBeenCalled()
    expect(simulateMultiHopSwap).not.toHaveBeenCalled()
    expect(queryWrapMapperConfig).not.toHaveBeenCalled()
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
  it.each(['missing_observations', 'stale_observations'] as const)(
    'waits indeterminately for %s and recovers only after a published generation',
    async (reason) => {
      let available = false
      const publication = { generation: undefined as string | undefined }
      vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) => {
        if (!available) throw new IndexerApiError(503, reason)
        return localQuote(from, to, amount, options)
      })
      vi.mocked(indexerClient.getRouteSolveProgress).mockImplementation(async () => ({
        stage: 'idle',
        done: 0,
        total: 0,
        label: '',
        cache_hit: true,
        updated_at_ms: Date.now(),
        quote_generation: publication.generation,
      }))
      const { queryClient } = renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
      fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '1' } })
      await waitFor(() => expect(screen.getByTestId('swap-price-data-loading')).toBeInTheDocument())
      expect(screen.getByTestId('swap-you-receive')).toHaveValue('')
      expect(screen.getByTestId('swap-you-receive')).toHaveAttribute('aria-busy', 'true')
      expect(screen.getByTestId('swap-submit')).toBeDisabled()
      await waitFor(() => expect(screen.getByTestId('swap-submit')).toHaveTextContent('Wait for price'))
      expect(screen.queryByTestId('swap-quote-retry-error')).not.toBeInTheDocument()
      expect(screen.queryByTestId('swap-quote-unavailable')).not.toBeInTheDocument()
      expect(screen.queryByTestId('swap-market-data-outage-banner')).not.toBeInTheDocument()
      expect(screen.queryByRole('alert')).not.toBeInTheDocument()
      available = true
      publication.generation = 'published-first'
      await queryClient.invalidateQueries({ queryKey: ['blockedQuoteGeneration'] })
      await waitFor(() => expect(screen.getByTestId('swap-you-receive')).toHaveValue('1'))
      expect(screen.queryByTestId('swap-price-data-loading')).not.toBeInTheDocument()
      expect(screen.getByTestId('swap-submit')).toBeEnabled()
      expect(simulateSwap).not.toHaveBeenCalled()
      expect(simulateMultiHopSwap).not.toHaveBeenCalled()
    }
  )
  it.each(['amount_too_small', 'protection', 'unsupported_semantics'] as const)(
    'keeps actual %s blockers visible rather than presenting them as pricing waits',
    async (reason) => {
      vi.mocked(indexerClient.getRouteSolve).mockRejectedValue(new IndexerApiError(422, reason))
      renderWithProviders(<SwapPage />)
      await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
      fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '1' } })
      await waitFor(() => expect(screen.getByTestId('swap-quote-error')).toBeInTheDocument())
      expect(screen.queryByTestId('swap-price-data-loading')).not.toBeInTheDocument()
      expect(screen.getByTestId('swap-submit')).toBeDisabled()
    }
  )
  it('keeps receive sizing in a data wait and publishes only the validated matching offer', async () => {
    let available = false
    vi.spyOn(indexerClient, 'getRouteSolveExactOutput').mockImplementation(async (from, to, target, options) => {
      if (!available) throw new IndexerApiError(503, 'missing_observations')
      return {
        ...localQuote(from, to, target, options),
        sizing_mode: 'local_pool_exact_output',
        required_amount_in: target,
        target_amount_out: target,
      }
    })
    const { queryClient } = renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
    fireEvent.change(screen.getByTestId('swap-you-receive'), { target: { value: '1' } })
    await waitFor(() => expect(screen.getByTestId('swap-price-data-loading')).toBeInTheDocument())
    expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('')
    expect(screen.getByTestId('swap-you-pay-amount')).toHaveAttribute('aria-busy', 'true')
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('1')
    expect(screen.queryByTestId('swap-receive-quote-retry-error')).not.toBeInTheDocument()
    available = true
    vi.mocked(indexerClient.getRouteSolveProgress).mockResolvedValue({
      stage: 'idle',
      done: 0,
      total: 0,
      label: '',
      cache_hit: true,
      updated_at_ms: Date.now(),
      quote_generation: 'published-receive',
    })
    await queryClient.invalidateQueries({ queryKey: ['blockedQuoteGeneration'] })
    await waitFor(() => expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('1'))
    expect(screen.queryByTestId('swap-price-data-loading')).not.toBeInTheDocument()
    expect(screen.getByTestId('swap-submit')).toBeEnabled()
  })
  it('clears the old wait immediately on a flip and shows the other direction’s valid quote', async () => {
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) => {
      if (from === LOCAL_FROM) throw new IndexerApiError(503, 'missing_observations')
      return localQuote(from, to, amount, options)
    })
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(screen.getByTestId('swap-price-data-loading')).toBeInTheDocument())
    fireEvent.click(screen.getByRole('button', { name: 'Swap pay and receive tokens' }))
    // The new direction can have its own initial wait; no obsolete rejection survives.
    expect(screen.queryByTestId('swap-quote-error')).not.toBeInTheDocument()
    await waitFor(() => expect(screen.getByTestId('swap-you-receive')).toHaveValue('1'))
  })
  it('immediately rechecks an expired cached amount within the global10s freshness window', async () => {
    let firstAmountCalls = 0
    let finish!: () => void
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) => {
      if (amount === '1000000' && ++firstAmountCalls === 2) {
        return new Promise((resolve) => {
          finish = () => resolve(localQuote(from, to, amount, options))
        })
      }
      const quote = localQuote(from, to, amount, options)
      if (amount === '1000000') quote.execution_metadata!.expires_at_ms = Date.now() + 1000
      return attachZeroMovementRiskFixture(quote)
    })
    const { queryClient } = renderWithProviders(<SwapPage />)
    queryClient.setDefaultOptions({ queries: { retry: false, gcTime: Infinity, staleTime: 10_000 } })
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeEnabled())
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '2' } })
    await waitFor(() => expect(screen.getByTestId('swap-you-receive')).toHaveValue('2'))
    await new Promise((resolve) => setTimeout(resolve, 1100))
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(firstAmountCalls).toBe(2), { timeout: 1000 })
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('')
    finish()
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeEnabled())
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
  it('does not warn about disconnected balances when the quote is available', async () => {
    useWalletStore.setState({ address: null, walletType: null })
    vi.mocked(getConnectedWallet).mockReturnValue(null)
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(screen.getByTestId('swap-you-receive')).toHaveValue('1'))
    expect(screen.getByTestId('swap-submit')).toHaveTextContent('Connect Wallet')
    expect(screen.queryAllByRole('alert')).toHaveLength(0)
    expect(screen.getByTestId('swap-price-details')).toHaveTextContent('Fair-reference deviation')
    expect(document.body).not.toHaveTextContent(/Balance unavailable|Need LUNC/i)
  })
  it('preserves unknown transaction status and makes a changed high quote explicit before signing', async () => {
    const router = await import('@/services/terraclassic/router')
    vi.mocked(router.executeMultiHopSwap).mockRejectedValueOnce(
      new Error('Broadcast status unknown — the transaction may still confirm.')
    )
    await ready()
    fireEvent.click(screen.getByTestId('swap-submit'))
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent(/unknown|confirm/i))
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) => {
      const q = localQuote(from, to, amount, options, ((BigInt(amount) * 89n) / 100n).toString())
      q.spot_amount_out = amount
      return attachZeroMovementRiskFixture(q)
    })
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '2' } })
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeEnabled())
    expect(screen.getByRole('alert')).toHaveTextContent(/unknown|confirm/i)
    fireEvent.click(screen.getByTestId('swap-submit'))
    expect(screen.getByTestId('swap-submit')).toHaveTextContent('Confirm Swap')
    expect(screen.getAllByRole('alert')).toHaveLength(1)
    expect(router.executeMultiHopSwap).toHaveBeenCalledTimes(1)
    vi.mocked(router.executeMultiHopSwap).mockResolvedValue('txhash123')
  })
  it.each(['paused', 'frozen', 'restricted'] as const)(
    'rejects current %s state at direct mutation entry and shows one actual blocker',
    async (kind) => {
      const { queryClient, rerender } = await ready()
      await act(async () => {
        queryClient.setQueryData(['pairPaused', LOCAL_PAIR], { paused: true })
        queryClient.setQueryData(['pairCodeIdFreeze', LOCAL_PAIR], { frozen: true })
        vi.mocked(useTradingBlacklist).mockReturnValue({
          ...TRADING_BLACKLIST_ALLOWED,
          blocked: true,
          message: 'Trading restricted.',
        })
        if (kind !== 'paused') queryClient.setQueryData(['pairPaused', LOCAL_PAIR], { paused: false })
        if (kind === 'restricted') queryClient.setQueryData(['pairCodeIdFreeze', LOCAL_PAIR], { frozen: false })
        rerender(<SwapPage />)
      })
      await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeDisabled())
      await expect(mutationEntry.run!()).rejects.toThrow(
        kind === 'paused'
          ? 'Trading is paused.'
          : kind === 'frozen'
            ? 'Trading is temporarily unavailable.'
            : 'Trading is restricted.'
      )
      expect(screen.getAllByRole('alert')).toHaveLength(1)
      expect(screen.queryByTestId('swap-impact-aside')).not.toBeInTheDocument()
      const router = await import('@/services/terraclassic/router')
      expect(router.executeMultiHopSwap).not.toHaveBeenCalled()
    }
  )
  it.each([40, 99])('preserves the %s%% route risk guard with Expert Mode', async (impact) => {
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) => {
      const q = localQuote(from, to, amount, options)
      q.spot_amount_out = ((BigInt(q.estimated_amount_out!) * 100n) / BigInt(100 - impact)).toString()
      return attachZeroMovementRiskFixture(q)
    })
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument())
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(indexerClient.getRouteSolve).toHaveBeenCalled())
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeDisabled())
    await expect(mutationEntry.run!()).rejects.toThrow('Quote risk is unavailable or exceeds protection.')
    expect(screen.getByTestId('swap-impact-aside')).toHaveAttribute('role', 'status')
    expect(document.body).not.toHaveTextContent(/Quote may change|Worst hop spread/i)
    useDexStore.setState({ expertMode: true })
    if (impact === 99) expect(screen.getByTestId('swap-submit')).toBeDisabled()
    else await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeEnabled())
  })
  it.each([false, true])(
    'shows one compact explanation and ignores diagnostic injection (tampered=%s)',
    async (tampered) => {
      vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) => {
        const q = localQuote(from, to, amount, options)
        q.spot_amount_out = '1010000'
        attachZeroMovementRiskFixture(q)
        if (tampered) (q.economic_attribution as Record<string, unknown>).dominant_cause = '<script>secret</script>'
        return q
      })
      if (tampered) {
        renderWithProviders(<SwapPage />)
        fireEvent.change(await screen.findByTestId('swap-you-pay-amount'), { target: { value: '1' } })
        await screen.findByText('Quote risk is unavailable. Refresh the quote.')
        expect(screen.getByTestId('swap-submit')).toBeDisabled()
        expect(document.body).not.toHaveTextContent('<script>secret</script>')
        return
      }
      await ready()
      expect(screen.getByTestId('swap-price-details')).toHaveTextContent('Fair-reference deviation 0.99%')
      expect(screen.getByTestId('swap-price-details')).toHaveTextContent('Fair-reference deviation 0.99%.')
      expect(screen.getByTestId('swap-price-details')).not.toHaveTextContent(/unavailable|breakdown|different pricing/i)
      expect(screen.getByTestId('swap-price-details').textContent?.match(/%/g)).toHaveLength(1)
      expect(screen.queryByTestId('swap-impact-aside')).not.toBeInTheDocument()
      expect(screen.getByTestId('swap-details')).not.toHaveAttribute('open')
      expect(screen.queryAllByRole('alert')).toHaveLength(0)
      expect(document.body).not.toHaveTextContent('<script>secret</script>')
    }
  )

  it('keeps a current 10.51% notice visible during an unchanged background refresh (#1489)', async () => {
    let pinned: ReturnType<typeof localQuote> | undefined
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) => {
      const q = localQuote(from, to, amount, options, '894900')
      q.spot_amount_out = '1000000'
      pinned = attachZeroMovementRiskFixture(q)
      return pinned
    })
    const { queryClient } = await ready()
    expect(screen.getByTestId('swap-impact-aside')).toHaveTextContent('Fair-reference deviation 10.51%')
    let finish: ((value: ReturnType<typeof localQuote>) => void) | undefined
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve
        })
    )
    await act(async () => {
      void queryClient.invalidateQueries({ queryKey: ['simulation'] })
    })
    await waitFor(() => expect(finish).toBeDefined())
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('0.8949')
    expect(screen.getByTestId('swap-impact-aside')).toHaveTextContent('Fair-reference deviation 10.51%')
    expect(screen.getByTestId('swap-impact-aside')).toHaveAttribute('role', 'status')
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
    await expect(mutationEntry.run!()).rejects.toThrow()
    const { executeMultiHopSwap } = await import('@/services/terraclassic/router')
    expect(executeMultiHopSwap).not.toHaveBeenCalled()
    await act(async () => {
      finish!(pinned!)
    })
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeEnabled())
    expect(screen.getByTestId('swap-impact-aside')).toHaveTextContent('Fair-reference deviation 10.51%')
  })
  it.each(['missing_observations', 'stale_observations'] as const)(
    'retains unexpired amount, route and notice through a %s refresh and subsequent recovery (#1499)',
    async (reason) => {
      vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) => {
        const q = localQuote(from, to, amount, options, '894900')
        q.spot_amount_out = '1000000'
        return attachZeroMovementRiskFixture(q)
      })
      const { queryClient } = await ready()
      const usefulCalls = vi.mocked(indexerClient.getRouteSolve).mock.calls.length
      vi.mocked(indexerClient.getRouteSolve).mockRejectedValue(new IndexerApiError(503, reason))
      await act(async () => {
        await queryClient.invalidateQueries({ queryKey: ['simulation'] })
      })
      await waitFor(() => expect(indexerClient.getRouteSolve).toHaveBeenCalledTimes(usefulCalls + 1))
      await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeDisabled())
      expect(screen.getByTestId('swap-you-receive')).toHaveValue('0.8949')
      expect(screen.getByTestId('swap-impact-aside')).toHaveTextContent('10.51%')
      expect(screen.queryByTestId('swap-price-data-loading')).not.toBeInTheDocument()
      expect(screen.getByTestId('swap-details')).toBeInTheDocument()
      expect(screen.getByTestId('swap-submit')).toBeDisabled()
      await expect(mutationEntry.run!()).rejects.toThrow()
      const { executeMultiHopSwap } = await import('@/services/terraclassic/router')
      expect(executeMultiHopSwap).not.toHaveBeenCalled()
      vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) =>
        localQuote(from, to, amount, options, '990000')
      )
      await act(async () => {
        await queryClient.invalidateQueries({ queryKey: ['simulation'] })
      })
      await waitFor(() => expect(screen.getByTestId('swap-you-receive')).toHaveValue('0.99'))
      await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeEnabled())
    }
  )

  it.each(['unsupported_semantics', 'protection', 'trading_blocked'] as const)(
    'revokes a settled positive quote immediately after a genuine %s refresh (#1499)',
    async (reason) => {
      const { queryClient } = await ready()
      expect(screen.getByTestId('swap-you-receive')).not.toHaveValue('')
      vi.mocked(indexerClient.getRouteSolve).mockRejectedValue(new IndexerApiError(422, reason))
      await act(async () => {
        await queryClient.invalidateQueries({ queryKey: ['simulation'] })
      })
      await waitFor(() => expect(screen.getByTestId('swap-quote-error')).toBeInTheDocument())
      expect(screen.getByTestId('swap-you-receive')).toHaveValue('')
      expect(screen.queryByTestId('swap-price-data-loading')).not.toBeInTheDocument()
      expect(screen.queryByTestId('swap-impact-aside')).not.toBeInTheDocument()
      expect(screen.getByTestId('swap-details')).toBeInTheDocument()
      expect(screen.getByTestId('swap-submit')).toBeDisabled()
      await expect(mutationEntry.run!()).rejects.toThrow()
      const { executeMultiHopSwap } = await import('@/services/terraclassic/router')
      expect(executeMultiHopSwap).not.toHaveBeenCalled()
    }
  )

  it('updates the notice at exact 10% only after a newly validated quote changes (#1489)', async () => {
    let output = '894900'
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) => {
      const q = localQuote(from, to, amount, options, output)
      q.spot_amount_out = '1000000'
      return attachZeroMovementRiskFixture(q)
    })
    const { queryClient } = await ready()
    expect(screen.getByTestId('swap-impact-aside')).toHaveTextContent('10.51%')
    for (const [next, visible, percent] of [
      ['900000', false, '10.00%'],
      ['900001', false, '<10%'],
      ['899999', true, '>10%'],
      ['894900', true, '10.51%'],
    ] as const) {
      output = next
      await act(async () => {
        await queryClient.invalidateQueries({ queryKey: ['simulation'] })
      })
      await waitFor(() => expect(screen.getByTestId('swap-you-receive')).toHaveValue(String(Number(next) / 1_000_000)))
      if (visible) expect(screen.getByTestId('swap-impact-aside')).toHaveTextContent(percent)
      else {
        expect(screen.queryByTestId('swap-impact-aside')).not.toBeInTheDocument()
        expect(screen.getByTestId('swap-price-details')).toHaveTextContent(percent)
      }
    }
  })
  it('clears old-intent notice immediately and stops showing it at original expiry (#1489)', async () => {
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) => {
      const q = localQuote(from, to, amount, options, '894900')
      q.spot_amount_out = '1000000'
      q.execution_metadata!.expires_at_ms = Date.now() + 1000
      return attachZeroMovementRiskFixture(q)
    })
    const { queryClient } = await ready()
    expect(screen.getByTestId('swap-impact-aside')).toHaveTextContent('10.51%')
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(() => new Promise(() => {}))
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '2' } })
    expect(screen.queryByTestId('swap-impact-aside')).not.toBeInTheDocument()
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '1' } })
    await waitFor(() => expect(screen.getByTestId('swap-impact-aside')).toHaveTextContent('10.51%'))
    await act(async () => {
      void queryClient.invalidateQueries({ queryKey: ['simulation'] })
    })
    await waitFor(() => expect(screen.getByTestId('swap-you-receive')).toHaveValue(''), { timeout: 2000 })
    expect(screen.queryByTestId('swap-impact-aside')).not.toBeInTheDocument()
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
  })
  it('pins ordinary review beyond automatic refresh and revokes it on an intent edit', async () => {
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) => {
      const q = localQuote(from, to, amount, options, ((BigInt(amount) * 89n) / 100n).toString())
      q.spot_amount_out = amount
      q.execution_metadata!.expires_at_ms = Date.now() + 60_000
      return attachZeroMovementRiskFixture(q)
    })
    const { queryClient } = await ready()
    const { executeMultiHopSwap } = await import('@/services/terraclassic/router')
    vi.useFakeTimers({ toFake: ['Date', 'setTimeout', 'clearTimeout', 'setInterval', 'clearInterval'] })
    await act(async () => {
      await queryClient.invalidateQueries({ queryKey: ['simulation'] })
      await vi.advanceTimersByTimeAsync(1)
    })
    fireEvent.click(screen.getByTestId('swap-submit'))
    expect(screen.getByTestId('swap-submit')).toHaveTextContent('Confirm Swap')
    const reads = vi.mocked(indexerClient.getRouteSolve).mock.calls.length
    try {
      await act(async () => {
        await vi.advanceTimersByTimeAsync(25_000)
      })
      expect(indexerClient.getRouteSolve).toHaveBeenCalledTimes(reads)
      expect(screen.getByTestId('swap-submit')).toHaveTextContent('Confirm Swap')
      expect(screen.getByTestId('swap-submit')).toBeEnabled()
      fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '2' } })
      await act(async () => {
        await vi.advanceTimersByTimeAsync(500)
      })
      expect(vi.mocked(indexerClient.getRouteSolve).mock.calls.length).toBeGreaterThan(reads)
      expect(screen.getByTestId('swap-submit')).not.toHaveTextContent('Confirm Swap')
      expect(executeMultiHopSwap).not.toHaveBeenCalled()
    } finally {
      vi.useRealTimers()
    }
  })
  it('requires renewed consent just above 10% and after amount changes', async () => {
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) => {
      const q = localQuote(from, to, amount, options, ((BigInt(amount) * 89999n) / 100000n).toString())
      q.spot_amount_out = amount
      return attachZeroMovementRiskFixture(q)
    })
    await ready()
    fireEvent.click(screen.getByTestId('swap-submit'))
    expect(screen.getByTestId('swap-impact-aside')).toHaveTextContent('click again to confirm.')
    expect(screen.getByTestId('swap-impact-aside')).toHaveAttribute('role', 'status')
    await expect(mutationEntry.run!()).rejects.toThrow('Confirm the price difference first.')
    await expect(mutationEntry.run!()).rejects.toThrow('Confirm the price difference first.')
    expect(await import('@/services/terraclassic/router').then((m) => m.executeMultiHopSwap)).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Cancel', exact: true }))
    expect(screen.getByTestId('swap-impact-aside')).not.toHaveTextContent('click again to confirm.')
    fireEvent.click(screen.getByTestId('swap-submit'))
    fireEvent.change(screen.getByTestId('swap-you-pay-amount'), { target: { value: '2' } })
    await waitFor(() => expect(screen.getByTestId('swap-submit')).toBeEnabled())
    fireEvent.click(screen.getByTestId('swap-submit'))
    expect(screen.getByTestId('swap-impact-aside')).toHaveTextContent('click again to confirm.')
    expect(screen.getByTestId('swap-impact-aside')).toHaveAttribute('role', 'status')
    fireEvent.click(screen.getByTestId('swap-submit'))
    const router = await import('@/services/terraclassic/router')
    await waitFor(() => expect(router.executeMultiHopSwap).toHaveBeenCalledTimes(1))
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
