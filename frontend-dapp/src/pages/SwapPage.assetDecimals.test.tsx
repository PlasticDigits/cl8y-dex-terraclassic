import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import SwapPage from './SwapPage'
import { useWalletStore } from '@/hooks/useWallet'
import { queryContractTokenInfoImpl } from '@/test/lcdTokenInfoMock'
import { SIM_QUOTE_DEBOUNCE_MS } from '@/utils/quoteDebounce'
import { resetCw20TokenInfoInFlightForTests } from '@/utils/tokenDisplay'

const { TERRA_A, TERRA_B, WALLET, UST1 } = vi.hoisted(() => ({
  TERRA_A: 'terra1from00000000000000000000000000000001',
  TERRA_B: 'terra1to00000000000000000000000000000001',
  WALLET: 'terra1wallet000000000000000000000000000001',
  UST1: 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
}))

vi.mock('@/utils/pairCatalogRank', async (importActual) => {
  const actual = await importActual<typeof import('@/utils/pairCatalogRank')>()
  return {
    ...actual,
    defaultRetailSwapTokenPair: vi.fn(() => [TERRA_A, TERRA_B] as [string, string]),
  }
})

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
  getConnectedWallet: vi.fn().mockReturnValue({}),
}))
vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn(),
  getTokenBalance: vi.fn().mockResolvedValue('1000000000000000000'),
}))
vi.mock('@/services/terraclassic/pair', () => ({
  simulateSwap: vi.fn().mockResolvedValue({
    return_amount: '1000000000000000000',
    spread_amount: '100',
    commission_amount: '3000',
  }),
  simulateHybridSwap: vi.fn().mockResolvedValue({
    return_amount: '1000000000000000000',
    spread_amount: '100',
    commission_amount: '3000',
  }),
  swap: vi.fn().mockResolvedValue('txhash123'),
  reverseSimulateSwap: vi.fn().mockResolvedValue({
    offer_amount: '1000000000000000000',
    spread_amount: '100',
    commission_amount: '3000',
  }),
  getPairPaused: vi.fn().mockResolvedValue({ paused: false }),
  getPool: vi.fn().mockResolvedValue({
    assets: [
      { info: { token: { contract_addr: TERRA_A } }, amount: '1000000000000000000' },
      { info: { token: { contract_addr: TERRA_B } }, amount: '1000000000000000000' },
    ],
    total_share: '1000000',
  }),
}))
vi.mock('@/services/terraclassic/settings', () => ({
  getPairFeeConfig: vi.fn().mockResolvedValue({ fee_bps: 30, treasury: '' }),
}))
vi.mock('@/services/terraclassic/feeDiscount', () => ({
  getTraderDiscount: vi.fn().mockResolvedValue({ discount_bps: 0, needs_deregister: false, registration_epoch: null }),
  getRegistration: vi.fn().mockResolvedValue({ registered: false, tier_id: null, tier: null }),
}))
vi.mock('@/services/terraclassic/pairDiscountRegistry', () => ({
  getPairDiscountRegistry: vi.fn().mockResolvedValue(''),
}))
vi.mock('@/services/terraclassic/assetCodeIdFreeze', () => ({
  probePairCodeIdFreeze: vi.fn().mockResolvedValue({ frozen: false, verdict: 'tradable' }),
}))
vi.mock('@/services/terraclassic/swapRoutePreflight', () => ({
  preflightSwapRouteSpread: vi.fn().mockResolvedValue({
    worstSpreadPercent: '0.50',
    anyHopExceedsMaxSpread: false,
  }),
  enrichSwapOperationsWithHopMinReturns: vi.fn(async (operations: unknown[]) => operations),
}))
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
  wrapMapperFeeBps: () => 100,
  queryRateLimit: vi.fn().mockResolvedValue({
    config: { max_amount_per_window: '110000000000000', window_seconds: 86400 },
    current_window_start: null,
    amount_used: '0',
  }),
  getNativeForWrapped: () => null,
  wrapTreasuryMatchesEnv: () => true,
  wrapUnwrapFeeNote: () => '',
  netAfterWrapMapperFee: (amount: bigint) => amount,
}))
vi.mock('@/services/terraclassic/router', () => ({
  findRoute: vi.fn(),
  getAllTokens: vi.fn(),
  simulateMultiHopSwap: vi.fn().mockResolvedValue({ amount: '1000000' }),
  executeMultiHopSwap: vi.fn().mockResolvedValue('txhash123'),
  isDirectWrapUnwrap: vi.fn().mockReturnValue(null),
  findRouteWithNativeSupport: vi.fn().mockReturnValue(null),
  simulateNativeSwap: vi.fn().mockResolvedValue({ amount: '1' }),
  executeNativeSwap: vi.fn().mockResolvedValue('tx'),
}))
vi.mock('@/hooks/useTradingBlacklist', () => ({
  useTradingBlacklist: vi.fn(() => ({
    blocked: false,
    loading: false,
    reason: null,
  })),
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
import { getConnectedWallet } from '@/services/terraclassic/wallet'
import { queryContract, getTokenBalance } from '@/services/terraclassic/queries'
import { simulateSwap, reverseSimulateSwap } from '@/services/terraclassic/pair'
import * as indexerClient from '@/services/indexer/client'
import { TRADING_BLACKLIST_ALLOWED } from '@/test/tradingBlacklistMocks'
import { useTradingBlacklist } from '@/hooks/useTradingBlacklist'
import { defaultRetailSwapTokenPair } from '@/utils/pairCatalogRank'
import { USTR_CW20_ADDRESS } from '@/utils/tokenRegistry'

function seedPair(pay = TERRA_A, receive = TERRA_B) {
  vi.mocked(defaultRetailSwapTokenPair).mockReturnValue([pay, receive])
  vi.mocked(getAllPairsPaginated).mockResolvedValue({
    pairs: [
      {
        contract_addr: 'terra1pair00000000000000000000000000000001',
        liquidity_token: 'terra1lp000000000000000000000000000000001',
        asset_infos: [{ token: { contract_addr: pay } }, { token: { contract_addr: receive } }],
      },
    ],
  })
  vi.mocked(getAllTokens).mockReturnValue([pay, receive])
  vi.mocked(findRoute).mockReturnValue([
    {
      terra_swap: {
        offer_asset_info: { token: { contract_addr: pay } },
        ask_asset_info: { token: { contract_addr: receive } },
      },
    },
  ] as never)
}

describe('SwapPage unlisted CW20 decimals (#1255)', () => {
  beforeEach(() => {
    localStorage.clear()
    resetCw20TokenInfoInFlightForTests()
    vi.mocked(useTradingBlacklist).mockReturnValue(TRADING_BLACKLIST_ALLOWED)
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    seedPair()
    vi.mocked(queryContract).mockImplementation(queryContractTokenInfoImpl(18))
    vi.mocked(getTokenBalance).mockResolvedValue('1000000000000000000')
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('indexer unused'))
    vi.spyOn(indexerClient, 'getPair').mockRejectedValue(new Error('no indexer pair'))
    vi.spyOn(indexerClient, 'getTokens').mockResolvedValue([])
    vi.spyOn(indexerClient, 'getFeeDiscountHealth').mockResolvedValue({
      configured: true,
      fee_discount_registry_ok: true,
      consecutive_lcd_failures: 0,
    })
    vi.mocked(simulateSwap).mockClear()
  })

  it('T1: unlisted factory CW20 token_info 18 → simulateSwap offer 10^18', async () => {
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '1')
    await waitFor(
      () => {
        expect(simulateSwap).toHaveBeenCalled()
        const offer = vi.mocked(simulateSwap).mock.calls[0][2]
        expect(offer).toBe('1000000000000000000')
      },
      { timeout: 5000 }
    )
  })

  it('T2: type 1.5 on 18-dec unlisted → 15 × 10^17', async () => {
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '1.5')
    await waitFor(
      () => {
        expect(simulateSwap).toHaveBeenCalled()
        expect(vi.mocked(simulateSwap).mock.calls[0][2]).toBe('1500000000000000000')
      },
      { timeout: 5000 }
    )
  })

  it('T3: unlisted 6-dec typed 1 → 10^6', async () => {
    vi.mocked(queryContract).mockImplementation(queryContractTokenInfoImpl(6))
    vi.mocked(getTokenBalance).mockResolvedValue('1000000')
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '1')
    await waitFor(
      () => {
        expect(simulateSwap).toHaveBeenCalled()
        expect(vi.mocked(simulateSwap).mock.calls[0][2]).toBe('1000000')
      },
      { timeout: 5000 }
    )
  })

  it('T7: decimals in flight → no simulateSwap', async () => {
    vi.mocked(queryContract).mockImplementation(
      () =>
        new Promise(() => {
          /* hang */
        })
    )
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '1')
    await new Promise((r) => setTimeout(r, SIM_QUOTE_DEBOUNCE_MS + 80))
    expect(simulateSwap).not.toHaveBeenCalled()
    expect(screen.getByTestId('swap-submit')).toBeDisabled()
  })

  it('T8: Max on 18-dec unlisted raw 10^18 is human 1', async () => {
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.click(await screen.findByTestId('swap-pay-max'))
    const pay = screen.getByTestId('swap-you-pay-amount') as HTMLInputElement
    expect(pay.value).toBe('1')
  })

  it('hostile token_info.decimals 255 → no quote', async () => {
    vi.mocked(queryContract).mockImplementation(queryContractTokenInfoImpl(255))
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '1')
    await new Promise((r) => setTimeout(r, SIM_QUOTE_DEBOUNCE_MS + 80))
    expect(simulateSwap).not.toHaveBeenCalled()
    await waitFor(() => {
      expect(screen.getByTestId('swap-submit')).toHaveTextContent(/decimals unavailable/i)
    })
  })

  it('T4: listed USTR (18) types 1 → offer 10^18 without LCD', async () => {
    seedPair(USTR_CW20_ADDRESS, TERRA_B)
    vi.mocked(queryContract).mockImplementation(queryContractTokenInfoImpl(6))
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '1')
    await waitFor(
      () => {
        expect(simulateSwap).toHaveBeenCalled()
        expect(vi.mocked(simulateSwap).mock.calls[0][2]).toBe('1000000000000000000')
      },
      { timeout: 5000 }
    )
  })

  it('T5: listed 6-dec pay + 18-dec unlisted receive shows 10^18 as 1', async () => {
    seedPair(UST1, TERRA_B)
    vi.mocked(queryContract).mockImplementation(queryContractTokenInfoImpl(18))
    vi.mocked(simulateSwap).mockResolvedValue({
      return_amount: '1000000000000000000',
      spread_amount: '100',
      commission_amount: '3000',
    })
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '1')
    await waitFor(
      () => {
        expect(simulateSwap).toHaveBeenCalled()
        expect(vi.mocked(simulateSwap).mock.calls[0][2]).toBe('1000000')
      },
      { timeout: 5000 }
    )
    await waitFor(() => {
      const receive = screen.getByTestId('swap-you-receive') as HTMLInputElement
      expect(receive.value).toMatch(/^1(\.0+)?$/)
    })
  })

  it('T6: reverse quote 18-dec ask uses ask raw 10^18', async () => {
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    const receive = screen.getByTestId('swap-you-receive') as HTMLInputElement
    await user.clear(receive)
    await user.type(receive, '1')
    await waitFor(
      () => {
        expect(reverseSimulateSwap).toHaveBeenCalled()
        expect(vi.mocked(reverseSimulateSwap).mock.calls[0][2]).toBe('1000000000000000000')
      },
      { timeout: 5000 }
    )
  })
})
