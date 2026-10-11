import { executeMultiHopSwap } from '@/services/terraclassic/router'
import { localQuote } from '@/test/localRouteFixtures'
import * as indexerClient from '@/services/indexer/client'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { act, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import { TradeMarketOrderPanel } from '../TradeMarketOrderPanel'
import { useWalletStore } from '@/hooks/useWallet'
import { useDexStore } from '@/stores/dex'
import { DEFAULT_SLIPPAGE_TOLERANCE_PERCENT } from '@/utils/slippageProtectionCopy'
import { SIM_QUOTE_DEBOUNCE_MS } from '@/utils/quoteDebounce'
import { sounds } from '@/lib/sounds'
import type { PairInfo } from '@/types'
import { IndexerApiError } from '@/utils/routeQuoteErrors'

const PAIR_ADDR = 'terra1pair00000000000000000000000000000001'
const TERRA_A = 'terra1from00000000000000000000000000000001'
const TERRA_B = 'terra1to00000000000000000000000000000001'

const selectedPair: PairInfo = {
  contract_addr: PAIR_ADDR,
  liquidity_token: 'terra1lp000000000000000000000000000000001',
  asset_infos: [{ token: { contract_addr: TERRA_A } }, { token: { contract_addr: TERRA_B } }],
}

vi.mock('@/services/terraclassic/wallet', () => ({
  getConnectedWallet: vi.fn().mockReturnValue({}),
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

vi.mock('@/hooks/useLimitOrderEscrowBalance', () => ({
  useLimitOrderEscrowBalance: () => ({ data: '10000000000', isLoading: false, isError: false }),
}))

vi.mock('@/hooks/useNativeUlunaBalance', () => ({
  useNativeUlunaBalance: () => ({ data: '10000000000', isLoading: false, isError: false }),
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
  swap: vi.fn().mockResolvedValue('txhash'),
}))

vi.mock('@/services/terraclassic/router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/terraclassic/router')>()
  return {
    ...actual,
    simulateMultiHopSwap: vi.fn().mockResolvedValue({ amount: '1000000' }),
    executeMultiHopSwap: vi.fn().mockResolvedValue('txhash'),
  }
})

vi.mock('@/services/terraclassic/swapRoutePreflight', () => ({
  preflightSwapRouteSpread: vi.fn().mockResolvedValue({
    worstSpreadPercent: '0.50',
    anyHopExceedsMaxSpread: false,
  }),
  enrichSwapOperationsWithHopMinReturns: vi.fn(async (operations: unknown[]) => operations),
  computeDirectHybridMinReturn: vi.fn().mockResolvedValue('900000'),
}))

vi.mock('@/services/terraclassic/transactions', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/services/terraclassic/transactions')>()),
  executeCw20AllowanceThen: vi.fn(async (_a, _b, _c, _d, fn: () => Promise<string>) => fn()),
  estimateMarketPairSwapSequenceUlunaFeesTotal: vi.fn().mockReturnValue(1000000n),
}))

vi.mock('@/services/indexer/client', () => {
  const from = 'terra1from00000000000000000000000000000001'
  const to = 'terra1to00000000000000000000000000000001'
  const pair = 'terra1pair00000000000000000000000000000001'
  return {
    getRouteSolve: vi.fn().mockResolvedValue({
      token_in: from,
      token_out: to,
      hops: [{ pair, offer_token: from, ask_token: to }],
      router_operations: [
        {
          terra_swap: {
            offer_asset_info: { token: { contract_addr: from } },
            ask_asset_info: { token: { contract_addr: to } },
            hybrid: {
              pool_input: '800000',
              book_input: '200000',
              max_maker_fills: 8,
              book_start_hint: null,
            },
          },
        },
      ],
      quote_kind: 'indexer_hybrid_lcd',
      estimated_amount_out: '1000000',
      spot_amount_out: '1010000',
      slippage_percent: '1.00',
      intermediate_tokens: [from, to],
    }),
    postRouteSolve: vi.fn().mockRejectedValue(new Error('indexer unavailable')),
    getTokens: vi.fn().mockResolvedValue([]),
    getRouteSolveProgress: vi.fn(),
  }
})

import * as pair from '@/services/terraclassic/pair'

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn(), playSuccess: vi.fn(), playError: vi.fn() },
}))

function renderPanel(overrides?: { isPaused?: boolean; address?: string | null }) {
  if (overrides?.address === null) {
    useWalletStore.setState({ address: null, walletType: null, error: null })
  }
  return renderWithProviders(
    <TradeMarketOrderPanel
      pairAddr={PAIR_ADDR}
      selectedPair={selectedPair}
      pairs={[selectedPair]}
      side="ask"
      isPaused={overrides?.isPaused ?? false}
    />
  )
}

describe('TradeMarketOrderPanel slippage presets (GitLab #528)', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getRouteSolveProgress).mockResolvedValue({
      stage: 'idle',
      done: 0,
      total: 0,
      label: '',
      cache_hit: true,
      updated_at_ms: Date.now(),
    })
    vi.mocked(indexerClient.getTokens).mockResolvedValue(
      selectedPair.asset_infos.map((a) => ({
        contract_address: 'token' in a ? a.token.contract_addr : null,
        denom: 'native_token' in a ? a.native_token.denom : null,
        decimals: 6,
        symbol: 'fixture',
        token_type: 'cw20',
      })) as never
    )
    vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, constraints) =>
      localQuote(from, to, amount, constraints)
    )
    vi.useFakeTimers({ shouldAdvanceTime: true })
    vi.clearAllMocks()
    useDexStore.setState({ slippageTolerance: DEFAULT_SLIPPAGE_TOLERANCE_PERCENT })
    useWalletStore.setState({
      address: 'terra1wallet000000000000000000000000001',
      walletType: 'simulated',
      error: null,
    })
  })

  afterEach(() => {
    vi.useRealTimers()
    useDexStore.setState({ slippageTolerance: DEFAULT_SLIPPAGE_TOLERANCE_PERCENT })
  })

  async function openAdvanced(user: ReturnType<typeof userEvent.setup>) {
    await user.click(screen.getByTestId('trade-market-advanced-toggle'))
  }

  it.each(['missing_observations', 'stale_observations'] as const)(
    'waits for %s until a validated quote follows the first published generation',
    async (reason) => {
      let available = false
      vi.mocked(indexerClient.getRouteSolve).mockImplementation(async (from, to, amount, options) => {
        if (!available) throw new IndexerApiError(503, reason)
        return localQuote(from, to, amount, options)
      })
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
      const { queryClient } = renderPanel()
      await user.type(screen.getByTestId('limit-order-escrow-amount-input'), '1')
      await act(async () => {
        await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)
      })
      await waitFor(() => expect(screen.getByTestId('trade-market-price-data-loading')).toBeInTheDocument())
      expect(screen.queryByTestId('trade-market-quote-error')).not.toBeInTheDocument()
      expect(screen.queryByTestId('trade-market-expected-receive')).not.toBeInTheDocument()
      expect(screen.getByTestId('trade-market-submit')).toBeDisabled()
      expect(screen.getByTestId('trade-market-submit')).toHaveTextContent('Wait for price')
      available = true
      vi.mocked(indexerClient.getRouteSolveProgress).mockResolvedValue({
        stage: 'idle',
        done: 0,
        total: 0,
        label: '',
        cache_hit: true,
        updated_at_ms: Date.now(),
        quote_generation: 'published-trade',
      })
      await act(async () => {
        await queryClient.invalidateQueries({ queryKey: ['blockedQuoteGeneration'] })
        await vi.advanceTimersByTimeAsync(1)
      })
      await waitFor(() => expect(screen.getByTestId('trade-market-submit')).toBeEnabled())
      expect(screen.queryByTestId('trade-market-price-data-loading')).not.toBeInTheDocument()
      expect(screen.getByTestId('trade-market-expected-receive')).toHaveTextContent(/^1\.0+\s/)
    }
  )
  it.each(['amount_too_small', 'protection', 'unsupported_semantics'] as const)(
    'still shows a genuine %s blocker',
    async (reason) => {
      vi.mocked(indexerClient.getRouteSolve).mockRejectedValue(new IndexerApiError(422, reason))
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
      renderPanel()
      await user.type(screen.getByTestId('limit-order-escrow-amount-input'), '1')
      await act(async () => {
        await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)
      })
      await waitFor(() => expect(screen.getByTestId('trade-market-quote-error')).toBeInTheDocument())
      expect(screen.queryByTestId('trade-market-price-data-loading')).not.toBeInTheDocument()
      expect(screen.getByTestId('trade-market-submit')).toBeDisabled()
    }
  )

  it('hides chips on the default Market path; aligned group appears under Advanced (#693 T8)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    renderPanel()
    expect(screen.queryByTestId('trade-market-slippage-presets')).not.toBeInTheDocument()
    await openAdvanced(user)
    expect(screen.getByTestId('trade-market-slippage-presets')).toBeInTheDocument()
    expect(screen.getByTestId('trade-market-slippage-preset-0.5')).toBeInTheDocument()
    expect(screen.getByTestId('trade-market-slippage-preset-1')).toBeInTheDocument()
    expect(screen.getByTestId('trade-market-slippage-preset-5')).toHaveClass('tab-glass-active')
    expect(screen.getAllByTestId('trade-market-slippage-presets')).toHaveLength(1)
  })

  it('updates the store and active chip on click (and plays press sound)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    renderPanel()
    await openAdvanced(user)

    await user.click(screen.getByTestId('trade-market-slippage-preset-0.5'))
    expect(useDexStore.getState().slippageTolerance).toBe(0.5)
    expect(screen.getByTestId('trade-market-slippage-preset-0.5')).toHaveClass('tab-glass-active')
    expect(screen.getByTestId('trade-market-slippage-preset-5')).toHaveClass('tab-glass-inactive')
    expect(sounds.playButtonPress).toHaveBeenCalled()

    await user.click(screen.getByTestId('trade-market-slippage-preset-1'))
    expect(useDexStore.getState().slippageTolerance).toBe(1)

    await user.click(screen.getByTestId('trade-market-slippage-preset-5'))
    expect(useDexStore.getState().slippageTolerance).toBe(5)
  })

  it('still renders chips when paused or disconnected (not the money CTA)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    renderPanel({ isPaused: true, address: null })
    await openAdvanced(user)
    expect(screen.getByTestId('trade-market-slippage-preset-0.5')).toBeEnabled()
    expect(screen.getByTestId('trade-market-submit')).toBeInTheDocument()
  })

  it('submits max_spread "0.005" after selecting 0.5% (default snapshot stays 0.05)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    renderPanel()
    await openAdvanced(user)

    await user.click(screen.getByTestId('trade-market-slippage-preset-0.5'))
    await user.type(screen.getByTestId('limit-order-escrow-amount-input'), '1')
    await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)
    await waitFor(() => expect(screen.getByTestId('trade-market-submit')).toBeEnabled())
    expect(screen.getByTestId('swap-confirm-max-spread')).toHaveTextContent('0.5%')

    await user.click(screen.getByTestId('trade-market-submit'))
    await waitFor(() => expect(executeMultiHopSwap).toHaveBeenCalled())
    const swapArgs = vi.mocked(executeMultiHopSwap).mock.calls.at(-1)
    expect(swapArgs?.[4]).toBe('0.005')
    expect(swapArgs?.[8]).toMatchObject({ max_spread: '0.005', generation: 'fixture1' })
    expect(pair.swap).not.toHaveBeenCalled()
  })
})
