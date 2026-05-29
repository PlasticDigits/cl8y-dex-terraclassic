import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import LimitOrdersPage from './LimitOrdersPage'
import { renderWithProviders } from '@/test-utils'
import * as factory from '@/services/terraclassic/factory'
import * as indexerClient from '@/services/indexer/client'
import type { IndexerPair } from '@/types'

const PAIR = 'terra1pair0000000000000000000000000000000001'

const mockIndexerPair: IndexerPair = {
  pair_address: PAIR,
  asset_0: { symbol: 'AAA', contract_addr: 'terra1aaa0000000000000000000000000000001', denom: null, decimals: 6 },
  asset_1: { symbol: 'BBB', contract_addr: 'terra1bbb0000000000000000000000000000002', denom: null, decimals: 6 },
  lp_token: 'terra1lp000000000000000000000000000000001',
  fee_bps: 30,
  volume_quote_24h: '0',
  is_active: true,
}

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn(), playSuccess: vi.fn(), playError: vi.fn() },
}))

vi.mock('@/services/terraclassic/factory', () => ({
  getAllPairsPaginated: vi.fn(),
}))

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getPair: vi.fn(),
    getTrades: vi.fn(),
    getPairLimitBookPage: vi.fn(),
    getPairLimitPlacements: vi.fn(),
    getPairLimitCancellations: vi.fn(),
  }
})

vi.mock('@/services/terraclassic/pair', () => ({
  getPairPaused: vi.fn().mockResolvedValue({ paused: false }),
  placeLimitOrderWithAllowance: vi.fn(),
  cancelLimitOrder: vi.fn(),
}))

vi.mock('@/services/terraclassic/wallet', () => ({
  getConnectedWallet: vi.fn().mockReturnValue(null),
}))

vi.mock('@/services/terraclassic/settings', () => ({
  getPairFeeConfig: vi.fn().mockResolvedValue({ fee_bps: 30, treasury: 'terra1treasury0000000000000000000001' }),
}))

vi.mock('@/services/terraclassic/queries', () => ({
  queryContract: vi.fn().mockResolvedValue({}),
  getTokenBalance: vi.fn().mockResolvedValue('0'),
}))

vi.mock('@/hooks/useLimitOrderMakerFeeRates', () => ({
  useLimitOrderMakerFeeRates: () => ({
    effectiveFeeBps: 30,
    makerPlacementFeeBps: 30,
    feeLoading: false,
    feeError: null,
  }),
}))

vi.mock('@/hooks/usePairLimitCancellations', () => ({
  usePairLimitCancellations: () => ({ data: [], isLoading: false, isError: false }),
}))

vi.mock('@/components/trade/LimitOrderLadderPanel', () => ({
  LimitOrderLadderPanel: () => null,
}))

async function selectLimitsPair(user: ReturnType<typeof userEvent.setup>) {
  await waitFor(() => expect(factory.getAllPairsPaginated).toHaveBeenCalled())
  const pairControl = await screen.findByLabelText('Trading pair')
  await user.click(pairControl)
  const listbox = await screen.findByRole('listbox')
  const pairOption = within(listbox).getByRole('option', { name: /terra1pa.*000001/i })
  await user.click(pairOption)
  await waitFor(() => expect(indexerClient.getPair).toHaveBeenCalledWith(PAIR))
}

describe('LimitOrdersPage', () => {
  beforeEach(() => {
    vi.mocked(factory.getAllPairsPaginated).mockResolvedValue({
      pairs: [
        {
          contract_addr: PAIR,
          liquidity_token: 'terra1lp000000000000000000000000000000001',
          asset_infos: [
            { token: { contract_addr: 'terra1aaa0000000000000000000000000000001' } },
            { token: { contract_addr: 'terra1bbb0000000000000000000000000000002' } },
          ],
        },
      ],
    })
    vi.mocked(indexerClient.getPair).mockResolvedValue(mockIndexerPair)
    vi.mocked(indexerClient.getTrades).mockResolvedValue([])
    vi.mocked(indexerClient.getPairLimitBookPage).mockResolvedValue({
      side: 'bid',
      orders: [],
      has_more: false,
      next_after_order_id: null,
    })
    vi.mocked(indexerClient.getPairLimitPlacements).mockResolvedValue([])
    vi.mocked(indexerClient.getPairLimitCancellations).mockResolvedValue([])
  })

  it('shows limits market-data outage banner when workspace indexer queries fail (GitLab #218)', async () => {
    const user = userEvent.setup()
    vi.mocked(indexerClient.getPair).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getTrades).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getPairLimitBookPage).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)

    const banner = await screen.findByTestId('limits-market-data-outage-banner')
    expect(banner.textContent).toMatch(/market data service unavailable/i)
    expect(banner.textContent).not.toMatch(/VITE_INDEXER_URL|127\.0\.0\.1/i)
    expect(banner.textContent).toMatch(/order book depth|recent trades/i)
    expect(banner.textContent).toMatch(/pool reserves/i)
    expect(await screen.findByTestId('trade-book-unavailable-bid')).toBeInTheDocument()
    expect(await screen.findByTestId('trade-book-unavailable-ask')).toBeInTheDocument()
  })

  it('shows outage banner when tape fails while pair metadata is cached (GitLab #218)', async () => {
    const user = userEvent.setup()
    vi.mocked(indexerClient.getTrades).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)

    expect(await screen.findByTestId('limits-market-data-outage-banner')).toBeInTheDocument()
  })

  it('does not show outage banner when indexer pair returns 404 (GitLab #218 / #177)', async () => {
    const user = userEvent.setup()
    vi.mocked(indexerClient.getPair).mockRejectedValue(new Error('Indexer API error: 404 Not Found'))
    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)

    await waitFor(() => {
      expect(indexerClient.getPair).toHaveBeenCalled()
    })
    expect(screen.queryByTestId('limits-market-data-outage-banner')).not.toBeInTheDocument()
  })

  it('blocks place when limit price cannot resolve during outage (GitLab #166 / #218)', async () => {
    const user = userEvent.setup()
    vi.mocked(indexerClient.getPair).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    vi.mocked(indexerClient.getTrades).mockRejectedValue(new Error('Indexer API error: 502 Bad Gateway'))
    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)

    await screen.findByTestId('limits-market-data-outage-banner')
    const guard = await screen.findByTestId('limits-page-place-guard')
    expect(guard.textContent).toMatch(/cannot validate limit price|pool|indexer|resolving reference/i)
  })

  it('shows pair-switch loading while indexer workspace queries fetch (GitLab #218 / #180)', async () => {
    const user = userEvent.setup()
    let resolvePair!: (value: IndexerPair) => void
    vi.mocked(indexerClient.getPair).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolvePair = resolve
        })
    )
    renderWithProviders(<LimitOrdersPage />, { route: '/limits' })
    await selectLimitsPair(user)
    expect(await screen.findByTestId('limits-pair-switch-loading')).toBeInTheDocument()
    resolvePair(mockIndexerPair)
    await waitFor(() => {
      expect(screen.queryByTestId('limits-pair-switch-loading')).not.toBeInTheDocument()
    })
  })
})
