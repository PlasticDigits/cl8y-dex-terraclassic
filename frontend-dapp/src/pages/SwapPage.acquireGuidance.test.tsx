import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import SwapPage from './SwapPage'
import { useWalletStore } from '@/hooks/useWallet'
import { UST1_RATE_SCALE } from '@/utils/ust1WindowMath'

const { UST1, TERRA_B, WALLET, WINDOW, VFDUSD } = vi.hoisted(() => ({
  UST1: 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
  TERRA_B: 'terra1to00000000000000000000000000000001',
  WALLET: 'terra1wallet000000000000000000000000000001',
  WINDOW: 'terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2',
  VFDUSD: 'terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3',
}))

vi.mock('@/utils/constants', async (importActual) => {
  const actual = await importActual<typeof import('@/utils/constants')>()
  return {
    ...actual,
    UST1_TOKEN_ADDRESS: UST1,
    VFDUSD_TOKEN_ADDRESS: VFDUSD,
    UST1_WINDOW_CONTRACT_ADDRESS: WINDOW,
    isUst1WindowEnabled: () => true,
  }
})

vi.mock('@/utils/pairCatalogRank', async (importActual) => {
  const actual = await importActual<typeof import('@/utils/pairCatalogRank')>()
  return {
    ...actual,
    defaultRetailSwapTokenPair: () => [UST1, TERRA_B] as [string, string],
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
  getConnectedWallet: vi.fn().mockReturnValue(null),
}))
vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn().mockResolvedValue({}),
  getTokenBalance: vi.fn().mockResolvedValue('0'),
}))
vi.mock('@/services/terraclassic/pair', () => ({
  simulateSwap: vi.fn().mockResolvedValue({
    return_amount: '1587000000000',
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
      { info: { token: { contract_addr: UST1 } }, amount: '1000000000000' },
      { info: { token: { contract_addr: TERRA_B } }, amount: '1000000000000' },
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
}))
vi.mock('@/services/terraclassic/router', () => ({
  findRoute: vi.fn(),
  getAllTokens: vi.fn(),
  simulateMultiHopSwap: vi.fn().mockResolvedValue({ amount: '1587000000000' }),
  executeMultiHopSwap: vi.fn().mockResolvedValue('txhash123'),
  isDirectWrapUnwrap: vi.fn().mockReturnValue(null),
  findRouteWithNativeSupport: vi.fn().mockReturnValue(null),
  simulateNativeSwap: vi.fn().mockResolvedValue({ amount: '1' }),
  executeNativeSwap: vi.fn().mockResolvedValue('tx'),
}))
vi.mock('@/services/terraclassic/ust1Window', () => ({
  getUst1EffectiveSwap: vi.fn(),
  executeUst1Window: vi.fn(),
  payTokenForDirection: (d: 'deposit' | 'withdraw') => (d === 'deposit' ? VFDUSD : UST1),
  paySymbolForDirection: (d: 'deposit' | 'withdraw') => (d === 'deposit' ? 'vFDUSD' : 'UST1'),
  receiveSymbolForDirection: (d: 'deposit' | 'withdraw') => (d === 'deposit' ? 'UST1' : 'vFDUSD'),
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
import { getUst1EffectiveSwap } from '@/services/terraclassic/ust1Window'
import * as indexerClient from '@/services/indexer/client'

const nowSec = Math.floor(Date.now() / 1000)

const healthyWindow = {
  fee_bps: 100,
  per_tx_ust1_limit: '1000000000',
  rolling_24h_ust1_limit: '10000000000',
  paused: false,
  rolling_window_start_sec: nowSec - 100,
  rolling_volume_ust1: '0',
  max_oracle_age_sec: 21_600,
  oracle: {
    rate: UST1_RATE_SCALE.toString(),
    last_update_sec: nowSec - 30,
    paused: false,
  },
}

function seedUst1Pair() {
  vi.mocked(getAllPairsPaginated).mockResolvedValue({
    pairs: [
      {
        contract_addr: 'terra1pair00000000000000000000000000000001',
        liquidity_token: 'terra1lp000000000000000000000000000000001',
        asset_infos: [{ token: { contract_addr: UST1 } }, { token: { contract_addr: TERRA_B } }],
      },
    ],
  })
  vi.mocked(getAllTokens).mockReturnValue([UST1, TERRA_B])
  vi.mocked(findRoute).mockReturnValue([
    {
      terra_swap: {
        offer_asset_info: { token: { contract_addr: UST1 } },
        ask_asset_info: { token: { contract_addr: TERRA_B } },
      },
    },
  ] as never)
}

describe('SwapPage acquire guidance (GitLab #678)', () => {
  beforeEach(() => {
    vi.mocked(getConnectedWallet).mockReturnValue(null)
    useWalletStore.setState({ address: null, walletType: null, error: null })
    seedUst1Pair()
    vi.mocked(getTokenBalance).mockResolvedValue('0')
    vi.mocked(getUst1EffectiveSwap).mockResolvedValue(healthyWindow as never)
    vi.spyOn(indexerClient, 'getRouteSolve').mockRejectedValue(new Error('Indexer API error: 400 Bad Request'))
  })

  it('A1: disconnected settled quote is quote-only with Connect Wallet', async () => {
    const user = userEvent.setup()
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '50000')
    await waitFor(() => {
      const receive = screen.getByTestId('swap-you-receive')
      if (receive.tagName === 'INPUT') {
        expect((receive as HTMLInputElement).value).not.toMatch(/^0(\.0+)?$/)
        expect((receive as HTMLInputElement).value).toMatch(/\d/)
      } else {
        expect(receive).not.toHaveTextContent('0.00')
      }
    })
    expect(await screen.findByTestId('swap-quote-only')).toHaveTextContent(/quote only/i)
    expect(screen.getByRole('button', { name: /connect wallet/i })).toBeEnabled()
    expect(screen.queryByText(/Min Received/i)).not.toBeInTheDocument()
    expect(screen.queryByTestId('swap-acquire-guidance')).not.toBeInTheDocument()
  })

  it('A2/A4: connected empty UST1 + 50k pay blocks submit and states window cannot mint that size', async () => {
    const user = userEvent.setup()
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '50000')
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /insufficient balance/i })).toBeDisabled()
    })
    const banner = await screen.findByTestId('swap-acquire-guidance')
    expect(banner).toHaveTextContent(/cannot mint this size/i)
    const guide = screen.getByTestId('swap-acquire-guide')
    expect(guide).toHaveAttribute('href', expect.stringMatching(/^\/ust1\?direction=deposit/))
    expect(guide.getAttribute('href')).not.toMatch(/49999/)
  })

  it('A3: UST1 shortfall under cap Guides to inverse deposit amount', async () => {
    const user = userEvent.setup()
    vi.mocked(getConnectedWallet).mockReturnValue({} as never)
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
    renderWithProviders(<SwapPage />)
    await waitFor(() => expect(screen.queryByText(/loading pairs/i)).not.toBeInTheDocument(), { timeout: 5000 })
    await user.type(screen.getByTestId('swap-you-pay-amount'), '500')
    const guide = await screen.findByTestId('swap-acquire-guide')
    expect(screen.getByTestId('swap-acquire-message')).toHaveTextContent(/Deposit about/)
    expect(guide).toHaveAttribute('href', expect.stringMatching(/^\/ust1\?direction=deposit&amount=/))
    expect(screen.getByRole('button', { name: /insufficient balance/i })).toBeDisabled()
  })
})
