import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen } from '@testing-library/react'
import { WalletIndexerHistoryPanel } from '@/components/trade/WalletIndexerHistoryPanel'
import { renderWithProviders } from '@/test-utils'
import * as indexerClient from '@/services/indexer/client'

const WALLET = 'terra1wallet000000000000000000000000000001'
const PAIR = 'terra1pair0000000000000000000000000000000001'

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getTraderTrades: vi.fn(),
    getTraderLimitFills: vi.fn(),
    getTraderLimitCancellations: vi.fn(),
  }
})

describe('WalletIndexerHistoryPanel (GitLab #352)', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getTraderTrades).mockResolvedValue([
      {
        id: 1,
        pair_address: PAIR,
        offer_asset: 'EMBER',
        ask_asset: 'CORAL',
        price: '1.2',
        commission_amount: '10',
        tx_hash: 'ABC123',
        block_timestamp: '2026-01-01T00:00:00Z',
      },
    ] as never)
    vi.mocked(indexerClient.getTraderLimitFills).mockResolvedValue([])
    vi.mocked(indexerClient.getTraderLimitCancellations).mockResolvedValue([])
  })

  it('wraps swap history table in horizontal scroll container', async () => {
    renderWithProviders(<WalletIndexerHistoryPanel walletAddress={WALLET} pairAddress={PAIR} sections={['swaps']} />)

    expect(await screen.findByTestId('wallet-history-table-scroll')).toBeInTheDocument()
    expect(screen.getByRole('columnheader', { name: 'Tx' })).toBeInTheDocument()
  })
})
