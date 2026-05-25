import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { WalletLuncBalance } from '../WalletLuncBalance'

vi.mock('@/hooks/useNativeUlunaBalance', () => ({
  useNativeUlunaBalance: vi.fn(),
}))

import { useNativeUlunaBalance } from '@/hooks/useNativeUlunaBalance'

const mockUseNativeUlunaBalance = vi.mocked(useNativeUlunaBalance)

const ADDR = 'terra1test000000000000000000000000000000000'

describe('WalletLuncBalance (GitLab #140)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('shows a loading state while the bank query is pending', () => {
    mockUseNativeUlunaBalance.mockReturnValue({
      isLoading: true,
      isError: false,
      data: undefined,
    } as ReturnType<typeof useNativeUlunaBalance>)
    render(<WalletLuncBalance address={ADDR} />)
    expect(screen.getByTestId('wallet-lunc-balance')).toHaveAttribute('aria-busy', 'true')
    expect(screen.getByText(/loading lunc balance/i)).toBeInTheDocument()
  })

  it('formats uluna raw amount as human LUNC', () => {
    mockUseNativeUlunaBalance.mockReturnValue({
      isLoading: false,
      isError: false,
      data: '1500000',
    } as ReturnType<typeof useNativeUlunaBalance>)
    render(<WalletLuncBalance address={ADDR} />)
    expect(screen.getByTestId('wallet-lunc-balance')).toHaveTextContent('1.500 LUNC')
  })

  it('shows an em dash when the query fails', () => {
    mockUseNativeUlunaBalance.mockReturnValue({
      isLoading: false,
      isError: true,
      data: undefined,
    } as ReturnType<typeof useNativeUlunaBalance>)
    render(<WalletLuncBalance address={ADDR} />)
    expect(screen.getByTestId('wallet-lunc-balance')).toHaveTextContent('— LUNC')
  })
})
