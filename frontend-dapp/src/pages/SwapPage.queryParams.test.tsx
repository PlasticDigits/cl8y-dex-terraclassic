import { describe, it, expect, vi, beforeEach } from 'vitest'
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, useLocation } from 'react-router-dom'
import { renderWithProviders } from '@/test-utils'
import SwapPage from './SwapPage'
import { useWalletStore } from '@/hooks/useWallet'

const { UST1, LISTED, USTR } = vi.hoisted(() => ({
  UST1: 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
  LISTED: 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v',
  USTR: 'terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv',
}))

const receiveTax = vi.hoisted(() => ({ buyBps: null as number | null }))
vi.mock('@/utils/constants', async (importActual) => ({
  ...(await importActual<typeof import('@/utils/constants')>()),
  WRAP_MAPPER_CONTRACT_ADDRESS: 'terra1mapper',
  UST1_TOKEN_ADDRESS: UST1,
  LUNC_C_TOKEN_ADDRESS: 'terra1luncwrapped',
  USTC_C_TOKEN_ADDRESS: 'terra1ustcwrapped',
  NATIVE_WRAPPED_PAIRS: { uluna: 'terra1luncwrapped', uusd: 'terra1ustcwrapped' },
  CL8Y_TOKEN_ADDRESS: 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3',
  TREASURY_CONTRACT_ADDRESS: 'terra1treasury',
  isNativeWrapEnabled: () => true,
}))
vi.mock('@/hooks/useCommunityTaxSellBps', () => ({
  useCommunityTaxSellBps: (token: string | null) => ({
    sellBps: null,
    buyBps: token === UST1 ? receiveTax.buyBps : null,
    isTaxToken: token === UST1 && receiveTax.buyBps != null,
    isLoading: false,
    detection: 'honest',
    extraDebitUnresolved: false,
  }),
  useCommunityTaxPreviewDebit: () => ({ debitRaw: null, previewUnresolved: false, isLoading: false }),
}))
vi.mock('@/hooks/useTradingBlacklist', () => ({
  useTradingBlacklist: () => ({ blocked: false, isLoading: false }),
}))

vi.mock('react-blockies', () => ({
  __esModule: true,
  default: function MockBlockies() {
    return null
  },
}))
vi.mock('@/services/terraclassic/factory', () => ({
  getAllPairsPaginated: vi.fn(),
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
  swap: vi.fn().mockResolvedValue('txhash123'),
  reverseSimulateSwap: vi.fn().mockResolvedValue({
    offer_amount: '1000000',
    spread_amount: '100',
    commission_amount: '3000',
  }),
  getPool: vi.fn().mockResolvedValue({
    assets: [
      { info: { native_token: { denom: 'uluna' } }, amount: '1000000' },
      { info: { token: { contract_addr: UST1 } }, amount: '1000000' },
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
vi.mock('@/services/terraclassic/wrapMapper', async (importActual) => ({
  ...(await importActual<typeof import('@/services/terraclassic/wrapMapper')>()),
  queryWrapMapperConfig: vi.fn().mockResolvedValue({
    governance: 'terra1gov',
    treasury: 'terra1treasury',
    paused: false,
    fee_wrap_bps: 0,
    fee_unwrap_bps: 0,
  }),
  queryPausedState: vi.fn().mockResolvedValue(false),
  checkRateLimitExceeded: vi.fn().mockResolvedValue(false),
  wrapTreasuryMatchesEnv: () => true,
}))
vi.mock('@/services/terraclassic/feeDiscount', () => ({
  getTraderDiscount: vi.fn().mockResolvedValue({
    discount_bps: 0,
    needs_deregister: false,
    registration_epoch: null,
  }),
  getRegistration: vi.fn().mockResolvedValue({ registered: false, tier_id: null, tier: null }),
}))
vi.mock('@/services/terraclassic/pairDiscountRegistry', () => ({
  getPairDiscountRegistry: vi.fn().mockResolvedValue(''),
}))
vi.mock('@/services/terraclassic/swapRoutePreflight', () => ({
  preflightSwapRouteSpread: vi.fn().mockResolvedValue({
    worstSpreadPercent: '0.50',
    anyHopExceedsMaxSpread: false,
  }),
  enrichSwapOperationsWithHopMinReturns: vi.fn(async (operations: unknown[]) => operations),
  computeDirectHybridMinReturn: vi.fn().mockResolvedValue('950000'),
}))
vi.mock('@/services/terraclassic/router', () => ({
  findRoute: vi.fn(),
  getAllTokens: vi.fn(),
  simulateMultiHopSwap: vi.fn().mockResolvedValue({ amount: '1000000' }),
  reverseSimulateMultiHopSwap: vi.fn().mockResolvedValue({ amount: '2000000' }),
  executeMultiHopSwap: vi.fn().mockResolvedValue('txhash123'),
  isDirectWrapUnwrap: vi.fn().mockReturnValue(null),
  findRouteWithNativeSupport: vi.fn().mockReturnValue(null),
  simulateNativeSwap: vi.fn().mockResolvedValue({ amount: '1' }),
  reverseSimulateNativeSwap: vi.fn().mockResolvedValue({ amount: '2000000' }),
  netCw20AfterNativeWrap: vi.fn(async (amount: bigint) => amount),
  netNativeAfterUnwrap: vi.fn(async (amount: bigint) => ({ receive: amount, routerMinReceiveBase: amount })),
  executeNativeSwap: vi.fn().mockResolvedValue('tx'),
}))
vi.mock('@/utils/swapReceiveQuote', () => ({ quoteSwapReceiveViaSolver: vi.fn() }))

vi.mock('@/lib/sounds', () => ({
  sounds: {
    playButtonPress: vi.fn(),
    playHover: vi.fn(),
    playSuccess: vi.fn(),
    playError: vi.fn(),
  },
}))

import { queryWrapMapperConfig } from '@/services/terraclassic/wrapMapper'
import { getAllPairsPaginated } from '@/services/terraclassic/factory'
import {
  findRoute,
  getAllTokens,
  findRouteWithNativeSupport,
  isDirectWrapUnwrap,
  reverseSimulateNativeSwap,
  reverseSimulateMultiHopSwap,
  simulateNativeSwap,
  simulateMultiHopSwap,
  executeNativeSwap,
} from '@/services/terraclassic/router'
import { quoteSwapReceiveViaSolver } from '@/utils/swapReceiveQuote'
import { computeDirectHybridMinReturn } from '@/services/terraclassic/swapRoutePreflight'
import { reverseSimulateSwap, simulateSwap, swap } from '@/services/terraclassic/pair'
import { getTokenBalance } from '@/services/terraclassic/queries'
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import * as indexerClient from '@/services/indexer/client'

function seedFactoryTokens() {
  vi.mocked(getAllPairsPaginated).mockResolvedValue({
    pairs: [
      {
        contract_addr: 'terra1pair00000000000000000000000000000001',
        liquidity_token: 'terra1lp000000000000000000000000000000001',
        asset_infos: [{ native_token: { denom: 'uluna' } }, { token: { contract_addr: UST1 } }],
      },
    ],
  })
  vi.mocked(getAllTokens).mockReturnValue(['uluna', 'uusd', UST1, LISTED, USTR])
  vi.mocked(findRoute).mockReturnValue([
    {
      terra_swap: {
        offer_asset_info: { native_token: { denom: 'uluna' } },
        ask_asset_info: { token: { contract_addr: UST1 } },
      },
    },
  ] as never)
}

async function waitForSwapReady() {
  await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
  await waitFor(() => expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toBeEnabled())
}

function seedCw20Route(multihop = false) {
  const tokens = multihop ? [LISTED, USTR, UST1] : [LISTED, UST1]
  const operations = tokens.slice(1).map((token, i) => ({
    terra_swap: {
      offer_asset_info: { token: { contract_addr: tokens[i] } },
      ask_asset_info: { token: { contract_addr: token } },
    },
  }))
  vi.mocked(getAllPairsPaginated).mockResolvedValue({
    pairs: operations.map((operation, i) => ({
      contract_addr: `terra1pair0000000000000000000000000000000${i}`,
      liquidity_token: `terra1lp00000000000000000000000000000000${i}`,
      asset_infos: [operation.terra_swap.offer_asset_info, operation.terra_swap.ask_asset_info],
    })),
  })
  vi.mocked(findRoute).mockReturnValue(operations)
  return operations
}

function renderWithSearch(route = '/?from=LUNC&to=USTC') {
  function SearchProbe() {
    const location = useLocation()
    return <span data-testid="receive-search-probe">{location.search}</span>
  }
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  render(
    <MemoryRouter initialEntries={[route]}>
      <QueryClientProvider client={queryClient}>
        <SearchProbe />
        <SwapPage />
      </QueryClientProvider>
    </MemoryRouter>
  )
  return queryClient
}

describe('SwapPage receive amount (Forgejo #1256)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(computeDirectHybridMinReturn).mockResolvedValue('950000')
    receiveTax.buyBps = null
    useWalletStore.setState({ address: null, walletType: null, error: null })
    vi.mocked(getConnectedWallet).mockReturnValue(null)
    vi.mocked(getTokenBalance).mockResolvedValue('100000000')
    vi.mocked(isDirectWrapUnwrap).mockReturnValue(null)
    vi.mocked(findRouteWithNativeSupport).mockReturnValue({
      operations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: LISTED } },
            ask_asset_info: { token: { contract_addr: UST1 } },
          },
        },
      ],
      needsWrapInput: true,
      needsUnwrapOutput: true,
    })
    vi.mocked(reverseSimulateNativeSwap).mockResolvedValue({ amount: '2000000' })
    vi.mocked(reverseSimulateMultiHopSwap).mockResolvedValue({ amount: '2000000' })
    vi.mocked(reverseSimulateSwap).mockResolvedValue({
      offer_amount: '2000000',
      spread_amount: '0',
      commission_amount: '0',
    })
    vi.mocked(simulateNativeSwap).mockResolvedValue({ amount: '1000000' })
    vi.mocked(simulateMultiHopSwap).mockResolvedValue({ amount: '1000000' })
    seedFactoryTokens()
    vi.mocked(quoteSwapReceiveViaSolver).mockImplementation(async (input) => ({
      offerAmount: '2000000',
      return_amount: input.targetRaw,
      spotAmountOut: receiveTax.buyBps ? '1000000' : input.targetRaw,
      routeSlippagePercent: '0.00',
      executeAmountOut: receiveTax.buyBps ? '1000000' : undefined,
      spread_amount: '0',
      commission_amount: '0',
      indexerOperations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: LISTED } },
            ask_asset_info: { token: { contract_addr: UST1 } },
          },
        },
      ],
      indexerIntermediateTokens: [LISTED, UST1],
    }))
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('indexer unavailable'))
  })

  it('native receive typing stays visible immediately and updates pay plus output URL', async () => {
    renderWithSearch()
    await waitForSwapReady()
    const receive = screen.getByTestId('swap-you-receive')
    expect(receive.tagName).toBe('INPUT')
    fireEvent.change(receive, { target: { value: '1.5' } })
    expect(receive).toHaveValue('1.5')
    expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('')
    await waitFor(() =>
      expect(quoteSwapReceiveViaSolver).toHaveBeenCalledWith(
        expect.objectContaining({ targetRaw: '1500000', fromToken: 'uluna', toToken: 'uusd' })
      )
    )
    await waitFor(() => expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('2'))
    await waitFor(() => expect(screen.getByTestId('receive-search-probe')).toHaveTextContent('exactField=output'))
    await waitFor(() => expect(screen.getByTestId('receive-search-probe')).toHaveTextContent('exactAmount=1.5'))
    expect(swap).not.toHaveBeenCalled()
  })

  it.each(['wrap', 'unwrap'] as const)(
    'direct %s accepts output amount through the native reverse service',
    async (direction) => {
      vi.mocked(isDirectWrapUnwrap).mockReturnValue(direction)
      const from = direction === 'wrap' ? 'uluna' : UST1
      const to = direction === 'wrap' ? UST1 : 'uluna'
      renderWithSearch(`/?from=${from}&to=${to}&exactField=output&exactAmount=1.5`)
      await waitForSwapReady()
      // Quote startup includes the production debounce and async token discovery;
      // a cold, bounded CI worker can exceed waitFor's default one-second window.
      await waitFor(
        () => expect(reverseSimulateNativeSwap).toHaveBeenCalledWith('1500000', from, to, expect.any(Array), undefined),
        { timeout: 5_000 }
      )
      await waitFor(() => expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('2'))
      expect(screen.getByTestId('swap-you-receive')).toHaveValue('1.5')
    }
  )

  it.each([false, true])('CW20 output intent asks the solver for a selected route (multihop %s)', async (multihop) => {
    seedCw20Route(multihop)
    renderWithSearch(`/?from=${LISTED}&to=${UST1}&exactField=output&exactAmount=1.5`)
    await waitForSwapReady()
    await waitFor(() =>
      expect(quoteSwapReceiveViaSolver).toHaveBeenCalledWith(
        expect.objectContaining({
          fromToken: LISTED,
          toToken: UST1,
          targetRaw: '1500000',
        })
      )
    )
    expect(reverseSimulateSwap).not.toHaveBeenCalled()
    expect(reverseSimulateMultiHopSwap).not.toHaveBeenCalled()
    await waitFor(() => expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('2'))
  })

  it('indexer-only output intent remains output-sided and asks the server', async () => {
    vi.mocked(findRoute).mockReturnValue(null)
    vi.mocked(findRouteWithNativeSupport).mockReturnValue(null)
    renderWithSearch(`/?from=${LISTED}&to=${UST1}&exactField=output&exactAmount=1.5`)
    await waitForSwapReady()
    await waitFor(() => expect(quoteSwapReceiveViaSolver).toHaveBeenCalled())
    expect(screen.getByTestId('receive-search-probe')).toHaveTextContent('exactField=output')
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('1.5')
  })

  it('positive targets below receive precision show an actionable finite error without querying', async () => {
    renderWithSearch('/?from=LUNC&to=USTC&exactField=output&exactAmount=0.0000001')
    await waitForSwapReady()
    await waitFor(() =>
      expect(screen.getByTestId('swap-receive-amount-error')).toHaveTextContent('below token precision')
    )
    expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('')
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('0.0000001')
    expect(screen.getByTestId('swap-you-pay-amount')).not.toHaveAttribute('aria-busy', 'true')
    expect(quoteSwapReceiveViaSolver).not.toHaveBeenCalled()
  })

  it('native server-only receive sizing loads mapper configuration and preserves output intent', async () => {
    vi.mocked(findRoute).mockReturnValue(null)
    vi.mocked(findRouteWithNativeSupport).mockReturnValue(null)
    renderWithSearch('/?from=LUNC&to=USTC&exactField=output&exactAmount=100')
    await waitForSwapReady()
    await waitFor(() => expect(queryWrapMapperConfig).toHaveBeenCalled())
    await waitFor(() =>
      expect(quoteSwapReceiveViaSolver).toHaveBeenCalledWith(expect.objectContaining({ targetRaw: '100000000' }))
    )
    await waitFor(() => expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('2'))
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('100')
  })

  it('mapper configuration failure exposes a finite disconnected error', async () => {
    vi.mocked(queryWrapMapperConfig).mockRejectedValueOnce(new Error('LCD unavailable'))
    vi.mocked(findRoute).mockReturnValue(null)
    vi.mocked(findRouteWithNativeSupport).mockReturnValue(null)
    renderWithSearch('/?from=LUNC&to=USTC&exactField=output&exactAmount=100')
    await waitForSwapReady()
    await waitFor(() =>
      expect(screen.getByTestId('swap-receive-quote-retry-error')).toHaveTextContent('configuration is unavailable')
    )
    expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('')
    expect(quoteSwapReceiveViaSolver).not.toHaveBeenCalled()
    expect(screen.getByTestId('retry-error-button')).toBeEnabled()
  })

  it('server-only single-hop CW20 submit preserves the requested floor when the fresh quote floor is weaker', async () => {
    vi.mocked(computeDirectHybridMinReturn).mockResolvedValue('900000')
    vi.mocked(findRoute).mockReturnValue(null)
    vi.mocked(findRouteWithNativeSupport).mockReturnValue(null)
    const wallet = 'terra1wallet000000000000000000000000000001'
    useWalletStore.setState({ address: wallet, walletType: 'dev' })
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    vi.mocked(quoteSwapReceiveViaSolver).mockResolvedValue({
      offerAmount: '2000000',
      return_amount: '1000000',
      spread_amount: '0',
      commission_amount: '0',
      indexerOperations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: LISTED } },
            ask_asset_info: { token: { contract_addr: UST1 } },
          },
        },
      ],
      indexerPairs: [
        {
          contract_addr: 'terra1verifiedpair',
          liquidity_token: 'terra1lp',
          asset_infos: [{ token: { contract_addr: LISTED } }, { token: { contract_addr: UST1 } }],
        },
      ],
    })
    renderWithSearch(`/?from=${LISTED}&to=${UST1}&exactField=output&exactAmount=1`)
    await waitForSwapReady()
    const submit = await screen.findByRole('button', { name: /^swap$/i })
    await waitFor(() => expect(submit).toBeEnabled())
    fireEvent.click(submit)
    await waitFor(() =>
      expect(swap).toHaveBeenCalledWith(
        wallet,
        LISTED,
        'terra1verifiedpair',
        '2000000',
        undefined,
        '0.05',
        undefined,
        expect.objectContaining({ minReturn: '950000' })
      )
    )
  })

  it('wallet context changes request fresh output sizing with the new trader', async () => {
    seedCw20Route()
    renderWithSearch(`/?from=${LISTED}&to=${UST1}&exactField=output&exactAmount=1`)
    await waitForSwapReady()
    await waitFor(() => expect(quoteSwapReceiveViaSolver).toHaveBeenCalledTimes(1))
    act(() => useWalletStore.setState({ address: 'terra1newtrader', walletType: 'dev' }))
    await waitFor(() =>
      expect(quoteSwapReceiveViaSolver).toHaveBeenLastCalledWith(
        expect.objectContaining({ quoteTrader: { trader: 'terra1newtrader' } })
      )
    )
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('1')
  })

  it('clearing receive clears both fields and removes the output amount from the URL', async () => {
    renderWithSearch('/?from=LUNC&to=USTC&exactField=output&exactAmount=1.5')
    await waitForSwapReady()
    await waitFor(() => expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('2'))
    fireEvent.change(screen.getByTestId('swap-you-receive'), { target: { value: '' } })
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('')
    expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('')
    await waitFor(() => expect(screen.getByTestId('receive-search-probe')).not.toHaveTextContent('exactAmount='))
  })

  it('flip keeps output intent and quotes the swapped assets', async () => {
    const user = userEvent.setup()
    renderWithSearch('/?from=LUNC&to=USTC&exactField=output&exactAmount=1.5')
    await waitForSwapReady()
    await waitFor(() => expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('2'))
    await user.click(screen.getByRole('button', { name: 'Swap pay and receive tokens' }))
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('1.5')
    await waitFor(() =>
      expect(quoteSwapReceiveViaSolver).toHaveBeenCalledWith(
        expect.objectContaining({ targetRaw: '1500000', fromToken: 'uusd', toToken: 'uluna' })
      )
    )
  })

  it.each(['swap-pay-max', 'swap-pay-frac-25', 'swap-pay-frac-50', 'swap-pay-frac-75', 'swap-pay-frac-100'])(
    '%s switches output editing back to pay intent',
    async (testId) => {
      seedCw20Route()
      useWalletStore.setState({ address: 'terra1wallet000000000000000000000000000001', walletType: 'dev' })
      vi.mocked(getConnectedWallet).mockReturnValue({} as never)
      renderWithSearch(`/?from=${LISTED}&to=${UST1}&exactField=output&exactAmount=1.5`)
      await waitForSwapReady()
      await waitFor(() => expect(screen.getByTestId(testId)).toBeEnabled())
      fireEvent.click(screen.getByTestId(testId))
      await waitFor(() => expect(screen.getByTestId('receive-search-probe')).not.toHaveTextContent('exactField=output'))
      expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue(
        testId === 'swap-pay-max' ? '100' : testId.split('-').at(-1)
      )
    }
  )

  it('hostile receive text is rejected and oversized numeric paste is capped', async () => {
    renderWithSearch()
    await waitForSwapReady()
    const receive = screen.getByTestId('swap-you-receive')
    fireEvent.change(receive, { target: { value: '1' } })
    fireEvent.change(receive, { target: { value: '1e9<script>' } })
    expect(receive).toHaveValue('1')
    fireEvent.change(receive, { target: { value: '123456789012345678901234567890' } })
    expect((receive as HTMLInputElement).value.length).toBeLessThanOrEqual(24)
  })

  it('rapid edits debounce the reverse request to the final target', async () => {
    renderWithSearch()
    await waitForSwapReady()
    for (const value of ['1', '12', '123'])
      fireEvent.change(screen.getByTestId('swap-you-receive'), { target: { value } })
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('123')
    await waitFor(() => expect(quoteSwapReceiveViaSolver).toHaveBeenCalledTimes(1))
    expect(quoteSwapReceiveViaSolver).toHaveBeenLastCalledWith(expect.objectContaining({ targetRaw: '123000000' }))
  })

  it('a stale reverse completion never supplies the pay amount for a newer target', async () => {
    let resolveOld!: (value: Awaited<ReturnType<typeof quoteSwapReceiveViaSolver>>) => void
    let resolveNew!: (value: Awaited<ReturnType<typeof quoteSwapReceiveViaSolver>>) => void
    vi.mocked(quoteSwapReceiveViaSolver)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveOld = resolve
          })
      )
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveNew = resolve
          })
      )
    renderWithSearch()
    await waitForSwapReady()
    fireEvent.change(screen.getByTestId('swap-you-receive'), { target: { value: '1' } })
    await waitFor(() => expect(quoteSwapReceiveViaSolver).toHaveBeenCalledTimes(1))
    fireEvent.change(screen.getByTestId('swap-you-receive'), { target: { value: '3' } })
    expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('')
    await waitFor(() => expect(quoteSwapReceiveViaSolver).toHaveBeenCalledTimes(2))
    await act(async () =>
      resolveOld({
        offerAmount: '2000000',
        return_amount: '1000000',
        spread_amount: '0',
        commission_amount: '0',
        indexerOperations: [],
      })
    )
    expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('')
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('3')
    expect(simulateNativeSwap).not.toHaveBeenCalled()
    expect(swap).not.toHaveBeenCalled()
    await act(async () =>
      resolveNew({
        offerAmount: '6000000',
        return_amount: '3000000',
        spread_amount: '0',
        commission_amount: '0',
        indexerOperations: [],
      })
    )
    await waitFor(() => expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('6'))
  })

  it('buy-tax output intent sends the net target for server inversion exactly once', async () => {
    receiveTax.buyBps = 1000
    seedCw20Route()
    renderWithSearch(`/?from=${LISTED}&to=${UST1}&exactField=output&exactAmount=0.9`)
    await waitForSwapReady()
    await waitFor(() =>
      expect(quoteSwapReceiveViaSolver).toHaveBeenCalledWith(expect.objectContaining({ targetRaw: '900000' }))
    )
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('0.9')
  })

  it('buy-tax receive target keeps the stronger fresh net floor with the reverse offer', async () => {
    vi.mocked(computeDirectHybridMinReturn).mockResolvedValue('980000')
    receiveTax.buyBps = 1000
    seedCw20Route()
    const wallet = 'terra1wallet000000000000000000000000000001'
    useWalletStore.setState({ address: wallet, walletType: 'dev' })
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    vi.mocked(simulateSwap).mockResolvedValue({ return_amount: '1000000', spread_amount: '0', commission_amount: '0' })
    renderWithSearch(`/?from=${LISTED}&to=${UST1}&exactField=output&exactAmount=0.9`)
    await waitForSwapReady()
    await waitFor(() => expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('2'))
    await waitFor(() =>
      expect(quoteSwapReceiveViaSolver).toHaveBeenCalledWith(
        expect.objectContaining({ quoteTrader: { trader: wallet } })
      )
    )
    const submit = await screen.findByRole('button', { name: /^swap$/i })
    await waitFor(() => expect(submit).toBeEnabled())
    fireEvent.click(submit)
    await waitFor(() =>
      expect(swap).toHaveBeenCalledWith(
        wallet,
        LISTED,
        expect.any(String),
        '2000000',
        undefined,
        '0.05',
        undefined,
        expect.objectContaining({ deadline: expect.any(Number), minReturn: '980000' })
      )
    )
  })

  it('native output intent executes the reverse pay amount with the receive floor', async () => {
    const wallet = 'terra1wallet000000000000000000000000000001'
    useWalletStore.setState({ address: wallet, walletType: 'dev' })
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    renderWithSearch('/?from=LUNC&to=USTC&exactField=output&exactAmount=1')
    await waitForSwapReady()
    await waitFor(() => expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('2'))
    const submit = await screen.findByRole('button', { name: /^swap$/i })
    await waitFor(() => expect(submit).toBeEnabled())
    fireEvent.click(submit)
    await waitFor(() =>
      expect(executeNativeSwap).toHaveBeenCalledWith(
        wallet,
        'uluna',
        'uusd',
        '2000000',
        expect.any(Array),
        '0.05',
        '950000',
        expect.any(Number),
        expect.any(Array)
      )
    )
  })

  it('reverse offers still enforce the connected wallet balance guard', async () => {
    seedCw20Route()
    useWalletStore.setState({ address: 'terra1wallet000000000000000000000000000001', walletType: 'dev' })
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    vi.mocked(getTokenBalance).mockResolvedValue('1000000')
    renderWithSearch(`/?from=${LISTED}&to=${UST1}&exactField=output&exactAmount=1`)
    await waitForSwapReady()
    await waitFor(() => expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('2'))
    await waitFor(() => expect(screen.getByRole('button', { name: /^insufficient balance$/i })).toBeDisabled())
    expect(swap).not.toHaveBeenCalled()
  })

  it('failed reverse quote clears pay and cannot start forward simulation', async () => {
    vi.mocked(quoteSwapReceiveViaSolver).mockRejectedValue(new Error('reverse quote unavailable'))
    renderWithSearch('/?from=LUNC&to=USTC&exactField=output&exactAmount=1.5')
    await waitForSwapReady()
    await waitFor(() => expect(quoteSwapReceiveViaSolver).toHaveBeenCalled())
    expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('')
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('1.5')
    expect(screen.getByTestId('swap-receive-quote-retry-error')).toHaveTextContent('Could not estimate output')
    expect(screen.getByTestId('retry-error-button')).toBeEnabled()
    expect(simulateNativeSwap).not.toHaveBeenCalled()
    expect(swap).not.toHaveBeenCalled()
  })

  it.each([
    'reverse quote unavailable',
    'Reverse query work budget exhausted',
    'Indexer API error: 429 Too Many Requests',
    'Indexer API error: 504 Gateway Timeout',
  ])('failed receive refetch (%s) clears executable pay without an automatic retry (#1389)', async (failure) => {
    seedCw20Route()
    useWalletStore.setState({ address: 'terra1wallet000000000000000000000000000001', walletType: 'dev' })
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    const queryClient = renderWithSearch(`/?from=${LISTED}&to=${UST1}&exactField=output&exactAmount=1`)
    await waitForSwapReady()
    await waitFor(() => expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('2'))
    const submit = await screen.findByRole('button', { name: /^swap$/i })
    await waitFor(() => expect(submit).toBeEnabled())
    let rejectRefetch!: (reason: Error) => void
    vi.mocked(quoteSwapReceiveViaSolver).mockImplementationOnce(
      () =>
        new Promise((_resolve, reject) => {
          rejectRefetch = reject
        })
    )
    act(() => {
      void queryClient.invalidateQueries({ queryKey: ['reverseSimulation'] })
    })
    await waitFor(() => expect(quoteSwapReceiveViaSolver).toHaveBeenCalledTimes(2))
    expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('')
    expect(screen.getByRole('button', { name: /calculating/i })).toBeDisabled()
    await act(async () => rejectRefetch(new Error(failure)))
    await waitFor(() => expect(screen.getByRole('button', { name: /^quote unavailable$/i })).toBeDisabled())
    expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('')
    expect(screen.getByTestId('swap-you-receive')).toHaveValue('1')
    expect(screen.getByTestId('retry-error-button')).toBeEnabled()
    // The query disables React Query retries and interval refresh after an error.
    // This also checks cached forward operations cannot enable an accidental resend.
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 1100))
    })
    expect(quoteSwapReceiveViaSolver).toHaveBeenCalledTimes(2)
    expect(swap).not.toHaveBeenCalled()
    expect(executeNativeSwap).not.toHaveBeenCalled()
  })
})

describe('SwapPage query params (GitLab #711)', () => {
  beforeEach(() => {
    vi.mocked(getConnectedWallet).mockReturnValue(null)
    useWalletStore.setState({ address: null, walletType: null, error: null })
    seedFactoryTokens()
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('indexer not used in this test'))
  })

  it('AC1: /?from=uluna&to=<listed cw20> selects LUNC and that token', async () => {
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${UST1}` })
    await waitForSwapReady()
    expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('LUNC')
    expect(screen.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('UST1')
  })

  it('AC1: waits for factory pairs so wrap natives do not drop a listed CW20', async () => {
    const factoryPairs = [
      {
        contract_addr: 'terra1pair00000000000000000000000000000001',
        liquidity_token: 'terra1lp000000000000000000000000000000001',
        asset_infos: [{ native_token: { denom: 'uluna' } }, { token: { contract_addr: UST1 } }],
      },
    ]
    let resolvePairs: (value: { pairs: typeof factoryPairs }) => void = () => {}
    vi.mocked(getAllPairsPaginated).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolvePairs = resolve
        })
    )
    vi.mocked(getAllTokens).mockImplementation((pairs: { asset_infos?: unknown[] }[] = []) =>
      pairs.length === 0 ? ['uluna', 'uusd'] : ['uluna', 'uusd', UST1, LISTED]
    )
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${UST1}` })
    expect(screen.getByText(/loading pairs/i)).toBeInTheDocument()
    resolvePairs({ pairs: factoryPairs })
    await waitForSwapReady()
    expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('LUNC')
    expect(screen.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('UST1')
  })

  it('AC2: Uniswap inputCurrency / outputCurrency', async () => {
    renderWithProviders(<SwapPage />, { route: `/?inputCurrency=uusd&outputCurrency=${UST1}` })
    await waitForSwapReady()
    expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('USTC')
    expect(screen.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('UST1')
  })

  it('AC2 one-sided: outputCurrency only defaults pay', async () => {
    renderWithProviders(<SwapPage />, { route: `/?outputCurrency=${UST1}` })
    await waitForSwapReady()
    expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('LUNC')
    expect(screen.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('UST1')
  })

  it('AC5: hostile values are ignored and not echoed in the trigger', async () => {
    renderWithProviders(<SwapPage />, { route: '/?from=javascript:alert(1)&to=%3Cscript%3E' })
    await waitForSwapReady()
    const pay = screen.getByRole('combobox', { name: 'Select token you pay' })
    const receive = screen.getByRole('combobox', { name: 'Select token you receive' })
    expect(pay).toHaveValue('LUNC')
    expect(receive).toHaveValue('USTC')
    expect((pay as HTMLInputElement).value).not.toMatch(/javascript/i)
    expect((receive as HTMLInputElement).value).not.toMatch(/script/i)
  })

  it('AC6: exactAmount prefills You Pay and does not auto-submit', async () => {
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${UST1}&exactAmount=1.5` })
    await waitForSwapReady()
    expect(screen.getByTestId('swap-you-pay-amount')).toHaveValue('1.5')
    expect(screen.getByRole('button', { name: /connect wallet/i })).toBeEnabled()
    expect(screen.queryByRole('button', { name: /^swap$/i })).not.toBeInTheDocument()
  })

  it('Q713-4: exactField=output prefills You Receive on a direct pair; execute stays quote-only', async () => {
    seedCw20Route()
    renderWithProviders(<SwapPage />, {
      route: `/?from=${LISTED}&to=${UST1}&exactAmount=1.5&exactField=output`,
    })
    await waitForSwapReady()
    await waitFor(() => {
      expect(screen.getByTestId('swap-you-receive')).toHaveValue('1.5')
    })
    expect(screen.getByRole('button', { name: /connect wallet/i })).toBeEnabled()
    expect(screen.queryByRole('button', { name: /^swap$/i })).not.toBeInTheDocument()
  })

  it('AC3: user picker after deep link is not snapped back; URL rewrites to from/to', async () => {
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />, { route: `/?inputCurrency=uluna&outputCurrency=${UST1}` })
    await waitForSwapReady()
    await waitFor(() => {
      expect(screen.getByTestId('swap-share-link')).toBeInTheDocument()
    })
    await user.click(screen.getByRole('combobox', { name: 'Select token you receive' }))
    await screen.findByRole('listbox', { name: 'Select token you receive' })
    await user.click(screen.getByRole('option', { name: /USTC/i }))
    await waitFor(() => {
      expect(screen.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('USTC')
    })
    expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('LUNC')
  })

  it('AC7: user can change receive after apply; query is not re-forced', async () => {
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${UST1}` })
    await waitForSwapReady()
    await user.click(screen.getByRole('combobox', { name: 'Select token you receive' }))
    await screen.findByRole('listbox', { name: 'Select token you receive' })
    await user.click(screen.getByRole('option', { name: /USTC/i }))
    await waitFor(() => {
      expect(screen.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('USTC')
    })
    expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('LUNC')
  })

  it('AC7: flip pay/receive after apply', async () => {
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />, { route: `/?from=uluna&to=${UST1}` })
    await waitForSwapReady()
    await user.click(screen.getByRole('button', { name: 'Swap pay and receive tokens' }))
    expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('UST1')
    expect(screen.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('LUNC')
  })

  it('QS-1: /?from=UST1&to=USTR selects those tokens (mixed case inbound)', async () => {
    renderWithProviders(<SwapPage />, { route: '/?from=ust1&to=USTR' })
    await waitForSwapReady()
    expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toHaveValue('UST1')
    expect(screen.getByRole('combobox', { name: 'Select token you receive' })).toHaveValue('USTR')
  })

  it('QS-2 / SH-1: bech32 inbound rewrites to symbols; Share shows logos and live aria-label', async () => {
    function SearchProbe() {
      const loc = useLocation()
      return <span data-testid="swap-search-probe">{loc.search}</span>
    }
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
    render(
      <MemoryRouter initialEntries={[`/?from=uluna&to=${UST1}`]}>
        <QueryClientProvider client={queryClient}>
          <SearchProbe />
          <SwapPage />
        </QueryClientProvider>
      </MemoryRouter>
    )
    await waitForSwapReady()
    await waitFor(() => {
      expect(screen.getByTestId('swap-search-probe').textContent).toMatch(/from=LUNC/)
      expect(screen.getByTestId('swap-search-probe').textContent).toMatch(/to=UST1/)
    })
    const share = screen.getByTestId('swap-share-link')
    expect(share).toHaveAttribute('aria-label', 'Share LUNC to UST1 swap link')
    expect(share).toHaveTextContent('Share')
    expect(share.querySelectorAll('img')).toHaveLength(2)
  })

  it('SH-1: flip updates Share aria-label', async () => {
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />, { route: '/?from=LUNC&to=UST1' })
    await waitForSwapReady()
    await waitFor(() => {
      expect(screen.getByTestId('swap-share-link')).toHaveAttribute('aria-label', 'Share LUNC to UST1 swap link')
    })
    await user.click(screen.getByRole('button', { name: 'Swap pay and receive tokens' }))
    expect(screen.getByTestId('swap-share-link')).toHaveAttribute('aria-label', 'Share UST1 to LUNC swap link')
  })
})
