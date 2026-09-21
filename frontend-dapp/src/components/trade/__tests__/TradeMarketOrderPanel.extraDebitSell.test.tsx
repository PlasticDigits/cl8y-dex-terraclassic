import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { renderWithProviders } from '@/test-utils'
import { TradeMarketOrderPanel } from '../TradeMarketOrderPanel'
import { useWalletStore } from '@/hooks/useWallet'
import { SIM_QUOTE_DEBOUNCE_MS } from '@/utils/quoteDebounce'
import type { PairInfo } from '@/types'

const { taxPreviewMock } = vi.hoisted(() => ({
  taxPreviewMock: { debitRaw: null as bigint | null },
}))

const PAIR_ADDR = 'terra1pair00000000000000000000000000000001'
const TERRA_A = 'terra1from00000000000000000000000000000001'
const TERRA_B = 'terra1to00000000000000000000000000000001'

const selectedPair: PairInfo = {
  contract_addr: PAIR_ADDR,
  liquidity_token: 'terra1lp000000000000000000000000000000001',
  asset_infos: [{ token: { contract_addr: TERRA_A } }, { token: { contract_addr: TERRA_B } }],
}

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
    debitRaw: taxPreviewMock.debitRaw,
    previewUnresolved: false,
    isLoading: false,
  }),
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
  getTokenBalance: vi.fn().mockResolvedValue('0'),
}))

vi.mock('@/hooks/useLimitOrderEscrowBalance', () => ({
  useLimitOrderEscrowBalance: () => ({ data: '1050000', isLoading: false, isError: false }),
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
          },
        },
      ],
      quote_kind: 'indexer_hybrid_lcd',
      estimated_amount_out: '1000000',
    }),
    postRouteSolve: vi.fn(),
    getTokens: vi.fn().mockResolvedValue([]),
  }
})

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn(), playSuccess: vi.fn(), playError: vi.fn() },
}))

import * as pair from '@/services/terraclassic/pair'

describe('TradeMarketOrderPanel extra-debit sell gate (#1267)', () => {
  beforeEach(() => {
    taxPreviewMock.debitRaw = null
    vi.useFakeTimers({ shouldAdvanceTime: true })
    vi.clearAllMocks()
    useWalletStore.setState({
      address: 'terra1wallet000000000000000000000000001',
      walletType: 'simulated',
      error: null,
    })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('T9: typing full tax CW20 balance disables Market sell', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )
    await user.type(screen.getByTestId('limit-order-escrow-amount-input'), '1.05')
    await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)
    await waitFor(() => expect(screen.getByTestId('trade-market-submit')).toBeDisabled())
    expect(screen.getByTestId('trade-market-submit')).toHaveTextContent(/insufficient balance/i)
    expect(pair.swap).not.toHaveBeenCalled()
  })

  it('T10 (#1285 AC3): Honest LCD debit === declared at full balance disables Market sell', async () => {
    taxPreviewMock.debitRaw = 1_050_000n
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )
    await user.type(screen.getByTestId('limit-order-escrow-amount-input'), '1.05')
    await vi.advanceTimersByTimeAsync(SIM_QUOTE_DEBOUNCE_MS + 50)
    await waitFor(() => expect(screen.getByTestId('trade-market-submit')).toBeDisabled())
    expect(screen.getByTestId('trade-market-submit')).toHaveTextContent(/insufficient balance/i)
    expect(pair.swap).not.toHaveBeenCalled()
  })

  it('T9 Max: extra-debit cap is below full balance', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    renderWithProviders(
      <TradeMarketOrderPanel
        pairAddr={PAIR_ADDR}
        selectedPair={selectedPair}
        pairs={[selectedPair]}
        side="ask"
        isPaused={false}
      />
    )
    await user.click(screen.getByTestId('limit-order-escrow-max'))
    const amount = screen.getByTestId('limit-order-escrow-amount-input') as HTMLInputElement
    expect(amount.value).toBe('1')
    expect(screen.getByTestId('trade-market-submit')).not.toHaveTextContent(/insufficient balance/i)
  })
})
