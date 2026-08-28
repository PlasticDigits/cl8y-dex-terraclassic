import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import { TradeMarketOrderPanel } from '../TradeMarketOrderPanel'
import { useWalletStore } from '@/hooks/useWallet'
import { UST1_RATE_SCALE } from '@/utils/ust1WindowMath'
import type { PairInfo } from '@/types'

const { UST1, TERRA_B, PAIR_ADDR, WALLET, WINDOW, VFDUSD } = vi.hoisted(() => ({
  UST1: 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
  TERRA_B: 'terra1to00000000000000000000000000000001',
  PAIR_ADDR: 'terra1pair00000000000000000000000000000001',
  WALLET: 'terra1wallet000000000000000000000000000001',
  WINDOW: 'terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2',
  VFDUSD: 'terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3',
}))

const selectedPair: PairInfo = {
  contract_addr: PAIR_ADDR,
  liquidity_token: 'terra1lp000000000000000000000000000000001',
  asset_infos: [{ token: { contract_addr: UST1 } }, { token: { contract_addr: TERRA_B } }],
}

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

vi.mock('@/services/terraclassic/wallet', () => ({
  getConnectedWallet: vi.fn().mockReturnValue({}),
}))

vi.mock('@/hooks/useLimitOrderEscrowBalance', () => ({
  useLimitOrderEscrowBalance: () => ({ data: '0', isLoading: false, isError: false }),
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

vi.mock('@/services/terraclassic/transactions', () => ({
  executeCw20AllowanceThen: vi.fn(async (_a, _b, _c, _d, fn: () => Promise<string>) => fn()),
  estimateMarketPairSwapSequenceUlunaFeesTotal: vi.fn().mockReturnValue(1000000n),
}))

vi.mock('@/services/terraclassic/ust1Window', () => ({
  getUst1EffectiveSwap: vi.fn(),
}))

vi.mock('@/services/indexer/client', () => ({
  getRouteSolve: vi.fn().mockRejectedValue(new Error('indexer unused')),
  postRouteSolve: vi.fn().mockRejectedValue(new Error('indexer unused')),
}))

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn(), playSuccess: vi.fn(), playError: vi.fn() },
}))

import { getUst1EffectiveSwap } from '@/services/terraclassic/ust1Window'

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

describe('TradeMarketOrderPanel acquire guidance (GitLab #678)', () => {
  beforeEach(() => {
    vi.mocked(getUst1EffectiveSwap).mockResolvedValue(healthyWindow as never)
    useWalletStore.setState({ address: WALLET, walletType: 'simulated', error: null })
  })

  it('A8: connected UST1 shortfall under cap shows Guide to /ust1', async () => {
    const user = userEvent.setup()
    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )
    await user.type(screen.getByTestId('limit-order-escrow-amount-input'), '500')
    const guide = await screen.findByTestId('trade-market-acquire-guide')
    expect(screen.getByTestId('trade-market-acquire-guidance')).toHaveTextContent(/Deposit about/)
    expect(guide).toHaveAttribute('href', expect.stringMatching(/^\/ust1\?direction=deposit&amount=/))
    expect(screen.getByTestId('trade-market-submit')).toBeDisabled()
  })

  it('A1 trade: disconnected quote is quote-only', async () => {
    const user = userEvent.setup()
    useWalletStore.setState({ address: null, walletType: null, error: null })
    const { getConnectedWallet } = await import('@/services/terraclassic/wallet')
    vi.mocked(getConnectedWallet).mockReturnValue(null)
    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )
    await user.type(screen.getByTestId('limit-order-escrow-amount-input'), '1')
    await waitFor(() => expect(screen.getByTestId('trade-market-submit')).toHaveTextContent(/connect wallet/i))
    expect(await screen.findByTestId('trade-market-quote-only')).toBeInTheDocument()
  })
})
