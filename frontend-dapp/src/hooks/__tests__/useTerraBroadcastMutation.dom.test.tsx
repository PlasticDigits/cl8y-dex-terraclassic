import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { TerraBroadcastPendingLink } from '@/components/ui/TerraBroadcastPendingLink'
import { resetTerraBroadcastScopeForTests } from '@/services/terraclassic/terraBroadcastScope'
import { executeTerraContract } from '@/services/terraclassic/transactions'
import { useTerraBroadcastMutation } from '../useTerraBroadcastMutation'

const mockBroadcastTx = vi.fn()
const mockPollTx = vi.fn()

vi.mock('@/services/terraclassic/wallet', () => ({
  getConnectedWallet: () => ({
    address: 'terra1sender',
    broadcastTx: mockBroadcastTx,
    pollTx: mockPollTx,
  }),
}))

function SwapSubmitFixture() {
  const swapMutation = useTerraBroadcastMutation({
    mutationFn: () => executeTerraContract('terra1sender', 'terra1contract', { swap: {} }),
  })

  return (
    <div>
      <button type="button" onClick={() => swapMutation.mutate()}>
        Swap
      </button>
      <TerraBroadcastPendingLink phase={swapMutation.phase} txHash={swapMutation.pendingTxHash} />
    </div>
  )
}

function wrapper({ children }: { children: React.ReactNode }) {
  const client = new QueryClient({ defaultOptions: { mutations: { retry: false } } })
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>
}

describe('useTerraBroadcastMutation DOM (GitLab #305 / #330)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetTerraBroadcastScopeForTests()
    mockBroadcastTx.mockResolvedValue('HASH305DOM')
    mockPollTx.mockResolvedValue({ txResponse: { code: 0, rawLog: '', logs: [] } })
  })

  it('paints TX link in the same commit as confirming before pollTx resolves', async () => {
    let resolvePoll!: (value: { txResponse: { code: number; rawLog: string; logs: [] } }) => void
    mockPollTx.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolvePoll = resolve
        })
    )

    render(<SwapSubmitFixture />, { wrapper })
    screen.getByRole('button', { name: 'Swap' }).click()

    await waitFor(() => {
      expect(screen.getByTestId('terra-broadcast-pending-tx')).toBeInTheDocument()
      expect(screen.getByText(/^TX:/)).toBeInTheDocument()
    })

    const pending = screen.getByTestId('terra-broadcast-pending-tx')
    expect(pending).toHaveAttribute('title', 'HASH305DOM')

    resolvePoll({ txResponse: { code: 0, rawLog: '', logs: [] } })
    await waitFor(() => expect(screen.queryByTestId('terra-broadcast-pending-tx')).toBeNull())
  })
})
