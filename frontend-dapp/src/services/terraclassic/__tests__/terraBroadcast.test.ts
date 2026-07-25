import { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { broadcastTerraExecuteContracts } from '../terraBroadcast'

const mockBroadcastTx = vi.fn()
const mockPollTx = vi.fn()

const mockWallet = {
  address: 'terra1sender',
  broadcastTx: mockBroadcastTx,
  pollTx: mockPollTx,
  getAuthInfo: vi.fn().mockResolvedValue({ accountNumber: 1n, sequence: 1n }),
}

beforeEach(() => {
  vi.clearAllMocks()
  mockBroadcastTx.mockResolvedValue('HASH1')
  mockPollTx.mockResolvedValue({ txResponse: { code: 0, rawLog: '', logs: [] } })
})

describe('broadcastTerraExecuteContracts (GitLab #127)', () => {
  it('rejects empty message list', async () => {
    await expect(broadcastTerraExecuteContracts(mockWallet as never, 'terra1sender', [])).rejects.toThrow(
      'No messages to broadcast'
    )
  })

  it('broadcasts single and multi-message txs through one code path', async () => {
    await broadcastTerraExecuteContracts(mockWallet as never, 'terra1sender', [
      { contract: 'terra1a', msg: { increase_allowance: { spender: 'terra1p', amount: '1' } } },
    ])
    expect(mockBroadcastTx).toHaveBeenCalledTimes(1)

    await broadcastTerraExecuteContracts(mockWallet as never, 'terra1sender', [
      { contract: 'terra1a', msg: { decrease_allowance: { spender: 'terra1p', amount: '1' } } },
      { contract: 'terra1b', msg: { decrease_allowance: { spender: 'terra1p', amount: '2' } } },
    ])
    expect(mockBroadcastTx).toHaveBeenCalledTimes(2)
  })

  it('maps Station false popup-closed WalletError (GitLab #208)', async () => {
    mockBroadcastTx.mockRejectedValueOnce(new Error('WalletError: User denied, extension popup was closed.'))
    await expect(
      broadcastTerraExecuteContracts(
        { ...mockWallet, id: WalletName.STATION, type: WalletType.EXTENSION } as never,
        'terra1sender',
        [{ contract: 'terra1a', msg: { swap: {} } }]
      )
    ).rejects.toThrow(/Station closed the signing popup/)
  })

  it('fires phase callbacks in order and skips confirming on broadcast failure (GitLab #305)', async () => {
    const phases: Array<{ phase: string; txHash?: string }> = []
    const onPhaseChange = vi.fn((phase: string, ctx?: { txHash?: string }) => {
      phases.push({ phase, txHash: ctx?.txHash })
    })

    await broadcastTerraExecuteContracts(
      mockWallet as never,
      'terra1sender',
      [{ contract: 'terra1a', msg: { swap: {} } }],
      { onPhaseChange }
    )

    expect(phases).toEqual([{ phase: 'signing' }, { phase: 'broadcasting' }, { phase: 'confirming', txHash: 'HASH1' }])

    vi.clearAllMocks()
    phases.length = 0
    mockBroadcastTx.mockRejectedValueOnce(new Error('User rejected the request'))

    await expect(
      broadcastTerraExecuteContracts(
        mockWallet as never,
        'terra1sender',
        [{ contract: 'terra1a', msg: { swap: {} } }],
        { onPhaseChange }
      )
    ).rejects.toThrow()

    expect(phases.some((p) => p.phase === 'confirming')).toBe(false)
    expect(phases.filter((p) => p.phase === 'signing')).toHaveLength(1)
  })

  it('enters confirming on poll failure without re-firing signing (GitLab #305)', async () => {
    const phases: string[] = []
    mockPollTx.mockRejectedValueOnce(new Error('not found'))

    await expect(
      broadcastTerraExecuteContracts(
        mockWallet as never,
        'terra1sender',
        [{ contract: 'terra1a', msg: { swap: {} } }],
        {
          onPhaseChange: (phase) => {
            phases.push(phase)
          },
        }
      )
    ).rejects.toThrow()

    expect(phases).toEqual(['signing', 'broadcasting', 'confirming'])
  })

  it('humanizes post-sign fee guard before generic user-denied copy (GitLab #127, #371)', async () => {
    mockBroadcastTx.mockRejectedValueOnce(
      new Error(
        'Wallet signed a fee far below what this dApp submitted (GitLab #127). Expected at least ~5665000 uluna; wallet returned ~3000 uluna.'
      )
    )
    await expect(
      broadcastTerraExecuteContracts(mockWallet as never, 'terra1sender', [
        { contract: 'terra1a', msg: { increase_allowance: { spender: 'terra1p', amount: '1' } } },
      ])
    ).rejects.toSatisfy((err: unknown) => {
      const msg = err instanceof Error ? err.message : String(err)
      return /Transaction fee mismatch/.test(msg) && !/GitLab #127/.test(msg) && !/uluna/.test(msg)
    })
  })
})
