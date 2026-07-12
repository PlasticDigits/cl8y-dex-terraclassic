import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { WalletIndexerHistoryPanel } from '@/components/trade/WalletIndexerHistoryPanel'
import { renderWithProviders } from '@/test-utils'
import * as indexerClient from '@/services/indexer/client'
import { formatNum } from '@/utils/formatAmount'

const WALLET = 'terra1wallet000000000000000000000000000001'
const PAIR = 'terra1pair0000000000000000000000000000000001'

vi.mock('@/services/indexer/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/indexer/client')>()
  return {
    ...actual,
    getTraderTrades: vi.fn(),
    getTraderLimitFills: vi.fn(),
    getTraderLimitCancellations: vi.fn(),
    fetchTraderHistoryCsv: vi.fn(),
    downloadTextAsFile: vi.fn(),
  }
})

describe('WalletIndexerHistoryPanel (GitLab #352 / #479)', () => {
  beforeEach(() => {
    vi.mocked(indexerClient.getTraderTrades).mockResolvedValue([
      {
        id: 1,
        pair_address: PAIR,
        offer_asset: 'EMBER',
        ask_asset: 'CORAL',
        offer_amount: '1000000',
        return_amount: '2000000',
        price: '1.2',
        commission_amount: '10',
        tx_hash: 'ABC123',
        block_timestamp: '2026-01-01T00:00:00Z',
      },
    ] as never)
    vi.mocked(indexerClient.getTraderLimitFills).mockResolvedValue([
      {
        id: 2,
        pair_address: PAIR,
        order_id: 7,
        side: 'bid',
        price: '1.1',
        token0_amount: '3000000',
        token1_amount: '4000000',
        commission_amount: '5',
        tx_hash: 'DEF456',
        block_timestamp: '2026-01-02T00:00:00Z',
      },
    ] as never)
    vi.mocked(indexerClient.getTraderLimitCancellations).mockResolvedValue([])
    vi.mocked(indexerClient.fetchTraderHistoryCsv).mockResolvedValue('id,offer_amount\n1,1000000\n')
    vi.mocked(indexerClient.downloadTextAsFile).mockClear()
  })

  it('wraps swap history table in horizontal scroll container', async () => {
    renderWithProviders(<WalletIndexerHistoryPanel walletAddress={WALLET} pairAddress={PAIR} sections={['swaps']} />)

    expect(await screen.findByTestId('wallet-history-table-scroll')).toBeInTheDocument()
    expect(screen.getByRole('columnheader', { name: 'Tx' })).toBeInTheDocument()
  })

  it('renders Amount in / Amount out for swap rows (#479)', async () => {
    renderWithProviders(<WalletIndexerHistoryPanel walletAddress={WALLET} pairAddress={PAIR} sections={['swaps']} />)

    expect(await screen.findByRole('columnheader', { name: 'Amount in' })).toBeInTheDocument()
    expect(screen.getByRole('columnheader', { name: 'Amount out' })).toBeInTheDocument()
    expect(screen.getByText(formatNum('1000000'))).toBeInTheDocument()
    expect(screen.getByText(formatNum('2000000'))).toBeInTheDocument()
  })

  it('renders Token0 / Token1 amounts for limit fills (#479)', async () => {
    renderWithProviders(<WalletIndexerHistoryPanel walletAddress={WALLET} pairAddress={PAIR} sections={['fills']} />)

    expect(await screen.findByRole('columnheader', { name: 'Token0' })).toBeInTheDocument()
    expect(screen.getByRole('columnheader', { name: 'Token1' })).toBeInTheDocument()
    expect(screen.getByText(formatNum('3000000'))).toBeInTheDocument()
    expect(screen.getByText(formatNum('4000000'))).toBeInTheDocument()
  })

  it('does not invent amount columns for cancellations', async () => {
    vi.mocked(indexerClient.getTraderLimitCancellations).mockResolvedValue([
      {
        id: 3,
        order_id: 9,
        tx_hash: 'GHI789',
        block_timestamp: '2026-01-03T00:00:00Z',
      },
    ] as never)

    renderWithProviders(<WalletIndexerHistoryPanel walletAddress={WALLET} pairAddress={PAIR} sections={['cancels']} />)

    expect(await screen.findByRole('columnheader', { name: 'Order' })).toBeInTheDocument()
    expect(screen.queryByRole('columnheader', { name: 'Amount in' })).not.toBeInTheDocument()
    expect(screen.queryByRole('columnheader', { name: 'Token0' })).not.toBeInTheDocument()
  })

  it('downloads CSV with server max limit and filename prefix on success', async () => {
    const user = userEvent.setup()
    renderWithProviders(<WalletIndexerHistoryPanel walletAddress={WALLET} pairAddress={PAIR} sections={['swaps']} />)

    await screen.findByTestId('wallet-history-download-csv')
    await user.click(screen.getByTestId('wallet-history-download-csv'))

    await waitFor(() => {
      expect(indexerClient.fetchTraderHistoryCsv).toHaveBeenCalledWith(
        'trades',
        WALLET,
        expect.objectContaining({ pair: PAIR, limit: indexerClient.TRADER_HISTORY_CSV_MAX_LIMIT })
      )
    })
    expect(indexerClient.downloadTextAsFile).toHaveBeenCalledWith(
      'swaps-terra1wallet00000000.csv',
      'id,offer_amount\n1,1000000\n'
    )
    expect(screen.queryByTestId('wallet-history-csv-error')).not.toBeInTheDocument()
  })

  it('shows inline CSV error and re-enables the button on failure (#479)', async () => {
    const user = userEvent.setup()
    vi.mocked(indexerClient.fetchTraderHistoryCsv).mockRejectedValueOnce(new Error('Failed to fetch'))

    renderWithProviders(<WalletIndexerHistoryPanel walletAddress={WALLET} pairAddress={PAIR} sections={['swaps']} />)

    const btn = await screen.findByTestId('wallet-history-download-csv')
    await user.click(btn)

    expect(await screen.findByTestId('wallet-history-csv-error')).toHaveTextContent(/CSV download failed/i)
    expect(indexerClient.downloadTextAsFile).not.toHaveBeenCalled()
    expect(btn).not.toBeDisabled()
  })

  it('returns null when wallet or pair is invalid', () => {
    const { container } = renderWithProviders(
      <WalletIndexerHistoryPanel walletAddress="" pairAddress={PAIR} sections={['swaps']} />
    )
    expect(container.querySelector('[data-testid="wallet-indexer-history"]')).toBeNull()
  })
})
