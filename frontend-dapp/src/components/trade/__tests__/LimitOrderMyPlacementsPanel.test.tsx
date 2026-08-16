import { describe, it, expect, vi, beforeEach } from 'vitest'
import { fireEvent, screen, waitFor } from '@testing-library/react'
import { LimitOrderMyPlacementsPanel } from '@/components/trade/LimitOrderMyPlacementsPanel'
import { renderWithProviders } from '@/test-utils'
import type { IndexerLimitPlacement, PairInfo } from '@/types'

const claimExpiredLimitOrder = vi.fn()
const claimExpiredLimitOrders = vi.fn()
const cancelLimitOrder = vi.fn()

vi.mock('@/services/terraclassic/pair', () => ({
  claimExpiredLimitOrder: (...args: unknown[]) => claimExpiredLimitOrder(...args),
  claimExpiredLimitOrders: (...args: unknown[]) => claimExpiredLimitOrders(...args),
  cancelLimitOrder: (...args: unknown[]) => cancelLimitOrder(...args),
}))

vi.mock('@/lib/sounds', () => ({
  sounds: { playSuccess: vi.fn(), playError: vi.fn() },
}))

vi.mock('@/utils/tokenDisplay', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/utils/tokenDisplay')>()
  return {
    ...actual,
    getTokenDisplaySymbol: () => 'UST1',
  }
})

const PAIR: PairInfo = {
  contract_addr: 'terra1pair00000000000000000000000000000001',
  liquidity_token: 'terra1lp000000000000000000000000000000001',
  asset_infos: [
    { token: { contract_addr: 'terra1aaa0000000000000000000000000000001' } },
    { token: { contract_addr: 'terra1bbb0000000000000000000000000000002' } },
  ],
}

const ACTIVE_ROW: IndexerLimitPlacement = {
  id: 1,
  pair_address: PAIR.contract_addr,
  block_height: 1,
  block_timestamp: '2026-01-01T00:00:00Z',
  tx_hash: 'ABC',
  order_id: 42,
  owner: 'terra1wallet',
  side: 'bid',
  price: '1.5',
  lifecycle_status: 'active',
}

function parkedRow(orderId: number, remaining = '1000000'): IndexerLimitPlacement {
  return {
    id: orderId + 100,
    pair_address: PAIR.contract_addr,
    block_height: 2,
    block_timestamp: '2026-01-02T00:00:00Z',
    tx_hash: 'DEF',
    order_id: orderId,
    owner: 'terra1wallet',
    side: 'bid',
    price: '1.0',
    lifecycle_status: 'parked_expired',
    remaining_escrow: remaining,
  }
}

function mockCancelMutation() {
  return {
    mutate: vi.fn(),
    mutateAsync: vi.fn(),
    isPending: false,
    isError: false,
    isSuccess: false,
    error: null,
    data: undefined,
    variables: undefined,
    reset: vi.fn(),
  }
}

describe('LimitOrderMyPlacementsPanel (GitLab #530)', () => {
  beforeEach(() => {
    claimExpiredLimitOrder.mockReset()
    claimExpiredLimitOrders.mockReset()
    cancelLimitOrder.mockReset()
    claimExpiredLimitOrder.mockResolvedValue('TX1')
    claimExpiredLimitOrders.mockResolvedValue('TXBATCH')
    cancelLimitOrder.mockResolvedValue('TXCANCEL')
    vi.stubGlobal(
      'confirm',
      vi.fn(() => true)
    )
  })

  it('emphasizes the active row when highlightOrderId matches (GitLab #161)', () => {
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="compact"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress="terra1wallet"
        rows={[ACTIVE_ROW]}
        isLoading={false}
        isWalletConnected
        isPairPaused={false}
        openWalletModal={vi.fn()}
        highlightOrderId={42}
      />
    )

    const li = screen.getByTestId('trade-placement-active-42')
    expect(li.className).toMatch(/shadow-\[0_0_0_2px/)
  })

  it('shows one-click cancel on active rows when cancel mutation is provided (#419)', () => {
    const cancelMutation = mockCancelMutation()
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="page"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress="terra1wallet"
        rows={[ACTIVE_ROW]}
        isLoading={false}
        isWalletConnected
        isPairPaused={false}
        openWalletModal={vi.fn()}
        cancelLimitOrderMutation={cancelMutation as never}
        cancellations={[]}
      />
    )

    const cancelBtn = screen.getByTestId('limits-page-cancel-placement-42')
    expect(cancelBtn).toHaveTextContent('Cancel')
    fireEvent.click(cancelBtn)
    expect(window.confirm).toHaveBeenCalled()
    expect(cancelMutation.mutate).toHaveBeenCalledWith(42)
  })

  it('disables row cancel while pair is paused (#419)', () => {
    const cancelMutation = mockCancelMutation()
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="compact"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress="terra1wallet"
        rows={[ACTIVE_ROW]}
        isLoading={false}
        isWalletConnected
        isPairPaused
        openWalletModal={vi.fn()}
        cancelLimitOrderMutation={cancelMutation as never}
      />
    )

    expect(screen.getByTestId('trade-cancel-placement-42')).toBeDisabled()
    expect(screen.getByTestId('trade-cancel-placement-42')).toHaveTextContent('Unavailable (pair paused)')
  })

  it('renders Claim all parked only when at least two parked rows exist (GitLab #253)', () => {
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="page"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress="terra1wallet"
        rows={[parkedRow(1)]}
        isLoading={false}
        isWalletConnected
        isPairPaused={false}
        openWalletModal={vi.fn()}
      />
    )

    expect(screen.queryByTestId('limits-page-claim-all-parked')).not.toBeInTheDocument()
  })

  it('shows Claim dust label for sub-threshold parked rows (#419)', () => {
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="page"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress="terra1wallet"
        rows={[parkedRow(7, '5')]}
        isLoading={false}
        isWalletConnected
        isPairPaused={false}
        openWalletModal={vi.fn()}
      />
    )

    expect(screen.getByText(/Dust — claim remaining/i)).toBeInTheDocument()
    expect(screen.getByTestId('limits-page-claim-expired-7')).toHaveTextContent('Claim dust')
  })

  it('shows Claim all parked and confirms batch claim for multiple rows', async () => {
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="page"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress="terra1wallet"
        rows={[parkedRow(10), parkedRow(11)]}
        isLoading={false}
        isWalletConnected
        isPairPaused={false}
        openWalletModal={vi.fn()}
      />
    )

    const claimAll = screen.getByTestId('limits-page-claim-all-parked')
    expect(claimAll).toHaveTextContent('Claim all parked (2)')

    fireEvent.click(claimAll)

    expect(window.confirm).toHaveBeenCalledWith(
      expect.stringContaining('Claim all 2 expired refund(s) in one transaction?')
    )
    await waitFor(() => {
      expect(claimExpiredLimitOrders).toHaveBeenCalledWith('terra1wallet', PAIR.contract_addr, [10, 11])
    })
  })

  it('disables Claim all parked while pair is paused', () => {
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="compact"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress="terra1wallet"
        rows={[parkedRow(1), parkedRow(2)]}
        isLoading={false}
        isWalletConnected
        isPairPaused
        openWalletModal={vi.fn()}
      />
    )

    expect(screen.getByTestId('trade-claim-all-parked')).toBeDisabled()
    expect(screen.getByTestId('trade-claim-all-parked')).toHaveTextContent('Unavailable (pair paused)')
  })

  it('labels already-indexed cancellation instead of mute Cancel (AC5)', () => {
    const cancelMutation = mockCancelMutation()
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="page"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress="terra1wallet"
        rows={[ACTIVE_ROW]}
        isLoading={false}
        isWalletConnected
        isPairPaused={false}
        openWalletModal={vi.fn()}
        cancelLimitOrderMutation={cancelMutation as never}
        cancellations={[
          {
            id: 1,
            pair_address: PAIR.contract_addr,
            block_height: 1,
            block_timestamp: 't',
            tx_hash: 'x',
            order_id: 42,
          },
        ]}
      />
    )
    const btn = screen.getByTestId('limits-page-cancel-placement-42')
    expect(btn).toBeDisabled()
    expect(btn).toHaveTextContent('Already cancelled')
    fireEvent.click(btn)
    expect(cancelMutation.mutate).not.toHaveBeenCalled()
  })

  it('report class: ●order #1 · Sell UST1 · 82.04… becomes Filled without Cancel (AC1/AC3)', () => {
    const cancelMutation = mockCancelMutation()
    const reportRow: IndexerLimitPlacement = {
      id: 1,
      pair_address: PAIR.contract_addr,
      block_height: 1,
      block_timestamp: '2026-08-15T14:21:43Z',
      tx_hash: 'ABC',
      order_id: 1,
      owner: 'terra1wallet',
      side: 'ask',
      price: '82.044004487226',
      lifecycle_status: 'active',
    }
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="compact"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress="terra1wallet"
        rows={[reportRow]}
        isLoading={false}
        isWalletConnected
        isPairPaused={false}
        openWalletModal={vi.fn()}
        cancelLimitOrderMutation={cancelMutation as never}
        lcdStatuses={{ 1: 'unknown' }}
        fills={[{ order_id: 1 }]}
      />
    )
    const li = screen.getByTestId('trade-placement-active-1')
    expect(li).toHaveAttribute('data-open-kind', 'filled')
    expect(li.textContent).toMatch(/order #1 · Sell UST1 · 82\.044004487226 · placed 2026-08-15T14:21:43/)
    expect(li.textContent).toMatch(/Filled/)
    const btn = screen.getByTestId('trade-cancel-placement-1')
    expect(btn).toBeDisabled()
    expect(btn).toHaveTextContent('Filled')
    fireEvent.click(btn)
    expect(cancelMutation.mutate).not.toHaveBeenCalled()
  })

  it('LCD ParkedRefund on an indexer-active row shows Claim, not Cancel (AC4)', () => {
    const cancelMutation = mockCancelMutation()
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="page"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress="terra1wallet"
        rows={[ACTIVE_ROW]}
        isLoading={false}
        isWalletConnected
        isPairPaused={false}
        openWalletModal={vi.fn()}
        cancelLimitOrderMutation={cancelMutation as never}
        lcdStatuses={{ 42: 'parked_refund' }}
      />
    )
    expect(screen.queryByTestId('limits-page-cancel-placement-42')).not.toBeInTheDocument()
    expect(screen.getByTestId('limits-page-claim-expired-42')).toHaveTextContent('Claim refund')
  })

  it('disables Cancel with Trading restricted copy (AC5)', () => {
    const cancelMutation = mockCancelMutation()
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="compact"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress="terra1wallet"
        rows={[ACTIVE_ROW]}
        isLoading={false}
        isWalletConnected
        isPairPaused={false}
        cancelDisabled
        openWalletModal={vi.fn()}
        cancelLimitOrderMutation={cancelMutation as never}
        lcdStatuses={{ 42: 'active' }}
      />
    )
    expect(screen.getByTestId('trade-cancel-placement-42')).toBeDisabled()
    expect(screen.getByTestId('trade-cancel-placement-42')).toHaveTextContent('Trading restricted')
  })

  it('disables Claim all parked when wallet disconnected', () => {
    renderWithProviders(
      <LimitOrderMyPlacementsPanel
        variant="page"
        pairAddr={PAIR.contract_addr}
        pair={PAIR}
        walletAddress=""
        rows={[parkedRow(1), parkedRow(2)]}
        isLoading={false}
        isWalletConnected={false}
        isPairPaused={false}
        openWalletModal={vi.fn()}
      />
    )

    expect(screen.getByTestId('limits-page-claim-all-parked')).toBeDisabled()
  })
})
