import { describe, expect, it, vi, beforeEach } from 'vitest'
import { screen, within } from '@testing-library/react'
import { renderWithProviders } from '@/test-utils'
import { getPairLimitBookPage } from '@/services/indexer/client'
import type { IndexerPair, PairInfo } from '@/types'
import { OrderBookPanel } from '../OrderBookPanel'

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getPairLimitBookPage: vi.fn(),
    getPairLimitCancellations: vi.fn().mockResolvedValue([]),
    getPairLimitPlacements: vi.fn().mockResolvedValue([]),
  }
})

const pair: IndexerPair = {
  pair_address: 'terra1pair00000000000000000000000000000001',
  asset_0: { symbol: 'EMBER', contract_addr: 'terra1ember', denom: null, decimals: 6 },
  asset_1: { symbol: 'CORAL', contract_addr: 'terra1coral', denom: null, decimals: 6 },
  lp_token: 'terra1lp',
  fee_bps: 30,
  is_active: true,
}

describe('OrderBookPanel', () => {
  beforeEach(() => {
    vi.mocked(getPairLimitBookPage).mockImplementation(async (_pair, side) => ({
      side,
      orders:
        side === 'bid'
          ? [
              { order_id: 7, owner: 'terra1maker', side, price: '0.826415278294723875', remaining: '139163969' },
              { order_id: 8, owner: 'terra1maker', side, price: '0.801', remaining: '50000000' },
            ]
          : [{ order_id: 9, owner: 'terra1maker', side, price: '1.16214523672018785', remaining: '104628895' }],
      has_more: false,
      next_after_order_id: null,
    }))
  })

  it('renders CEX-style columns with pair-aware labels and formatted rows', async () => {
    renderWithProviders(<OrderBookPanel pairAddress={pair.pair_address} pair={pair} />)

    expect(await screen.findByText('CORAL/EMBER')).toBeInTheDocument()
    expect(await screen.findByText('0.8264153')).toBeInTheDocument()
    expect(screen.getAllByText(/Order \/ price CORAL\/EMBER/).length).toBeGreaterThanOrEqual(2)
    expect(screen.getByText('Size CORAL')).toBeInTheDocument()
    expect(screen.getByText('Size EMBER')).toBeInTheDocument()

    const bids = screen.getByText('Bids').closest('.card-neo')
    expect(bids).toBeTruthy()
    expect(within(bids as HTMLElement).getAllByText('139.2').length).toBeGreaterThanOrEqual(2)
    expect(within(bids as HTMLElement).getByText('0.801')).toBeInTheDocument()
    expect(within(bids as HTMLElement).getByText('189.2')).toBeInTheDocument()

    const asks = screen.getByText('Asks').closest('.card-neo')
    expect(asks).toBeTruthy()
    expect(within(asks as HTMLElement).getByText('1.162145')).toBeInTheDocument()
    expect(within(asks as HTMLElement).getAllByText('104.6').length).toBeGreaterThanOrEqual(2)
  })

  it('shows per-row Edit / cancel for the connected wallet owner (GitLab #162)', async () => {
    const cancelMut = {
      mutate: vi.fn(),
      mutateAsync: vi.fn().mockResolvedValue('txhash'),
      isPending: false,
      variables: undefined as number | undefined,
      isError: false,
      error: null,
      isSuccess: false,
      data: undefined as string | undefined,
      reset: vi.fn(),
    }
    const onPrefill = vi.fn()
    const factoryPair: PairInfo = {
      contract_addr: pair.pair_address,
      liquidity_token: 'terra1lp',
      asset_infos: [{ token: { contract_addr: 'terra1ember' } }, { token: { contract_addr: 'terra1coral' } }],
    }

    renderWithProviders(
      <OrderBookPanel
        pairAddress={pair.pair_address}
        pair={pair}
        walletAddress="terra1maker"
        isWalletConnected
        isPairPaused={false}
        cancelLimitOrderMutation={cancelMut as never}
        onPrefillLimitTicket={onPrefill}
        factoryPair={factoryPair}
      />
    )

    expect(await screen.findByTestId('trade-book-edit-bid-7')).toBeInTheDocument()
    expect(screen.getByTestId('trade-book-cancel-bid-7')).toBeInTheDocument()
    expect(screen.getByTestId('trade-book-cancel-all-mine')).toBeInTheDocument()
  })
})
