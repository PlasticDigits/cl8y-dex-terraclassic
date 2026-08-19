import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { renderHook, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { ToastProvider } from '@/contexts/ToastContext'
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
    getAuthInfo: vi.fn().mockResolvedValue({ accountNumber: 1n, sequence: 1n }),
  }),
}))

function wrapper({ children }: { children: React.ReactNode }) {
  const client = new QueryClient({ defaultOptions: { mutations: { retry: false } } })
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>
}

function toastWrapper({ children }: { children: React.ReactNode }) {
  const client = new QueryClient({ defaultOptions: { mutations: { retry: false } } })
  return (
    <QueryClientProvider client={client}>
      <ToastProvider>{children}</ToastProvider>
    </QueryClientProvider>
  )
}

describe('useTerraBroadcastMutation (GitLab #305)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetTerraBroadcastScopeForTests()
    mockBroadcastTx.mockResolvedValue('HASH305')
    mockPollTx.mockResolvedValue({ txResponse: { code: 0, rawLog: '', logs: [] } })
  })

  it('tracks confirming phase and tx hash for scoped service calls', async () => {
    const { result } = renderHook(
      () =>
        useTerraBroadcastMutation({
          mutationFn: () => executeTerraContract('terra1sender', 'terra1contract', { swap: {} }),
        }),
      { wrapper }
    )

    result.current.mutate()

    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(result.current.data).toBe('HASH305')
    expect(mockBroadcastTx).toHaveBeenCalled()
  })

  it('exposes confirming phase and hash before pollTx settles (GitLab #330)', async () => {
    let resolvePoll!: (value: { txResponse: { code: number; rawLog: string; logs: [] } }) => void
    mockPollTx.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolvePoll = resolve
        })
    )

    const { result } = renderHook(
      () =>
        useTerraBroadcastMutation({
          mutationFn: () => executeTerraContract('terra1sender', 'terra1contract', { swap: {} }),
        }),
      { wrapper }
    )

    result.current.mutate()

    await waitFor(() => {
      expect(result.current.phase).toBe('confirming')
      expect(result.current.pendingTxHash).toBe('HASH305')
    })

    resolvePoll({ txResponse: { code: 0, rawLog: '', logs: [] } })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
  })

  it('pushes success toast when toastSuccess is set (GitLab #351)', async () => {
    const { result } = renderHook(
      () =>
        useTerraBroadcastMutation({
          toastSuccess: 'Swap submitted.',
          mutationFn: () => executeTerraContract('terra1sender', 'terra1contract', { swap: {} }),
        }),
      { wrapper: toastWrapper }
    )

    result.current.mutate()
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    await waitFor(() =>
      expect(document.querySelector('[data-testid="toast-success"]')).toHaveTextContent('Swap submitted.')
    )
  })

  it('clears phase after settlement', async () => {
    const { result } = renderHook(
      () =>
        useTerraBroadcastMutation({
          mutationFn: () => executeTerraContract('terra1sender', 'terra1contract', { swap: {} }),
        }),
      { wrapper }
    )

    result.current.mutate()
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(result.current.phase).toBeNull()
    expect(result.current.pendingTxHash).toBeNull()
  })
})
