import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import SwapPage from './SwapPage'
import { useWalletStore } from '@/hooks/useWallet'

const { TERRA_A, TERRA_B, WALLET } = vi.hoisted(() => ({
  TERRA_A: 'terra1from00000000000000000000000000000001',
  TERRA_B: 'terra1to00000000000000000000000000000001',
  WALLET: 'terra1wallet000000000000000000000000000001',
}))

vi.mock('@/utils/pairCatalogRank', async (importActual) => {
  const actual = await importActual<typeof import('@/utils/pairCatalogRank')>()
  return {
    ...actual,
    defaultRetailSwapTokenPair: () => [TERRA_A, TERRA_B] as [string, string],
  }
})

vi.mock('@/hooks/useCommunityTaxSellBps', () => ({
  useCommunityTaxSellBps: (addr: string | null | undefined) => ({
    sellBps: addr?.startsWith('terra1') ? 500 : null,
    buyBps: null,
    isTaxToken: !!addr?.startsWith('terra1'),
    isLoading: false,
    detection: addr?.startsWith('terra1') ? 'tax' : 'honest',
    extraDebitUnresolved: false,
  }),
  useCommunityTaxPreviewDebit: () => ({
    debitRaw: null,
    previewUnresolved: false,
    isLoading: false,
  }),
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
  getConnectedWallet: vi.fn().mockReturnValue({}),
}))
vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn(async (_addr: string, msg: unknown) => {
    if (msg && typeof msg === 'object' && 'token_info' in msg) {
      return { name: 'Dummy', symbol: 'DUM', decimals: 6, total_supply: '0' }
    }
    return {}
  }),
  getTokenBalance: vi.fn().mockResolvedValue('1050000'),
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
    offer_amount: '1050000',
    spread_amount: '100',
    commission_amount: '3000',
  }),
  getPairPaused: vi.fn().mockResolvedValue({ paused: false }),
  getPool: vi.fn().mockResolvedValue({
    assets: [
      { info: { token: { contract_addr: TERRA_A } }, amount: '1000000000' },
      { info: { token: { contract_addr: TERRA_B } }, amount: '1000000000' },
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
import { getTokenBalance } from '@/services/terraclassic/queries'
import { swap } from '@/services/terraclassic/pair'
import * as indexerClient from '@/services/indexer/client'
import { TRADING_BLACKLIST_ALLOWED } from '@/test/tradingBlacklistMocks'
import { useTradingBlacklist } from '@/hooks/useTradingBlacklist'

function seedPair() {
  vi.mocked(getAllPairsPaginated).mockResolvedValue({
    pairs: [
      {
        contract_addr: 'terra1pair00000000000000000000000000000001',
        liquidity_token: 'terra1lp000000000000000000000000000000001',
        asset_infos: [{ token: { contract_addr: TERRA_A } }, { token: { contract_addr: TERRA_B } }],
      },
    ],
  })
  vi.mocked(getAllTokens).mockReturnValue([TERRA_A, TERRA_B])
  vi.mocked(findRoute).mockReturnValue([
    {
      terra_swap: {
        offer_asset_info: { token: { contract_addr: TERRA_A } },
        ask_asset_info: { token: { contract_addr: TERRA_B } },
      },
    },
  ] as never)
}

describe('SwapPage extra-debit sell gate (#1267)', () => {
  beforeEach(() => {
    vi.mocked(useTradingBlacklist).mockReturnValue(TRADING_BLACKLIST_ALLOWED)
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    seedPair()
    vi.mocked(getTokenBalance).mockResolvedValue('1050000')
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('indexer unused'))
    vi.spyOn(indexerClient, 'getPair').mockRejectedValue(new Error('no pair'))
    vi.spyOn(indexerClient, 'getTokens').mockResolvedValue([])
    vi.spyOn(indexerClient, 'getFeeDiscountHealth').mockResolvedValue({
      configured: true,
      fee_discount_registry_ok: true,
      consecutive_lcd_failures: 0,
    })
  })

  it('T1: typing full CW20 balance disables Swap (no broadcast)', async () => {
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '1.05')
    expect(await screen.findByTestId('swap-submit')).toBeDisabled()
    expect(screen.getByTestId('swap-submit')).toHaveTextContent(/insufficient balance/i)
    expect(swap).not.toHaveBeenCalled()
  })

  it('T2: Max leaves extra-debit room so CTA is not Insufficient Balance', async () => {
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.click(await screen.findByTestId('swap-pay-max'))
    const pay = screen.getByTestId('swap-you-pay-amount') as HTMLInputElement
    expect(pay.value).toBe('1')
    expect(screen.getByTestId('swap-submit')).not.toHaveTextContent(/insufficient balance/i)
    expect(swap).not.toHaveBeenCalled()
  })
})
