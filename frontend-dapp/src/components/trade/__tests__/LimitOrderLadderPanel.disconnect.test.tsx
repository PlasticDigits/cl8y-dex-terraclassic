import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import { LimitOrderLadderPanel } from '../LimitOrderLadderPanel'
import { renderWithProviders } from '@/test-utils'

const PAIR = 'terra1pair0000000000000000000000000000000001'

vi.mock('@/hooks/useLimitOrderConfig', () => ({
  useLimitOrderConfig: () => ({ data: { max_batch_rungs: 20 }, isLoading: false }),
}))

vi.mock('@/hooks/useLimitLadderPlaceGates', () => ({
  useLimitLadderPlaceGates: () => ({
    canPlace: false,
    inlineGate: {
      canPlaceLimit: false,
      userMessage: 'Cannot verify balance.',
      tone: 'warning' as const,
    },
    batchMinUluna: 0n,
    gasSavingsUlunaVsSeparate: 0n,
    escrowBalanceQuery: { data: undefined, isLoading: false, isError: false },
    nativeUlunaQuery: { data: undefined, isLoading: false, isError: false },
  }),
}))

vi.mock('@/hooks/useLimitLadderPlacementPlan', () => ({
  useLimitLadderPlacementPlan: () => ({
    data: undefined,
    isLoading: false,
    isSuccess: false,
  }),
}))

vi.mock('@/hooks/useTradeBestBookPrices', () => ({
  useTradeBestBookPrices: () => ({
    bestBid: '1',
    bestAsk: '1.5',
    isLoading: false,
    isError: false,
    refetch: vi.fn(),
  }),
}))

describe('LimitOrderLadderPanel disconnected wallet (GitLab #494)', () => {
  const openWalletModal = vi.fn()

  beforeEach(() => {
    openWalletModal.mockClear()
  })

  it('renders create fields and Connect Wallet CTA when disconnected', async () => {
    const user = userEvent.setup()
    renderWithProviders(
      <LimitOrderLadderPanel
        pairAddress={PAIR}
        walletAddress={null}
        isWalletConnected={false}
        openWalletModal={openWalletModal}
        escrowToken="terra1token00000000000000000000000000001"
        escrowDecimals={6}
        token0Symbol="CORAL"
        token1Symbol="EMBER"
      />
    )

    expect(screen.getByTestId('limit-order-ladder-panel')).toBeInTheDocument()
    expect(screen.getByTestId('ladder-start-price')).toBeInTheDocument()
    expect(screen.getByTestId('ladder-end-price')).toBeInTheDocument()
    expect(screen.getByTestId('ladder-rung-count')).toBeInTheDocument()
    expect(screen.getByTestId('ladder-total-amount')).toBeInTheDocument()

    const submit = screen.getByTestId('ladder-place-submit')
    expect(submit).toBeEnabled()
    expect(submit).toHaveTextContent(/Connect Wallet/i)

    await user.click(submit)
    expect(openWalletModal).toHaveBeenCalledTimes(1)
  })
})
