import { WalletName, WalletType } from '@goblinhunt/cosmes/wallet'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { broadcastTerraExecuteContracts } from '../terraBroadcast'

const mockBroadcastTx = vi.fn()
const mockPollTx = vi.fn()

const mockWallet = {
  address: 'terra1sender',
  broadcastTx: mockBroadcastTx,
  pollTx: mockPollTx,
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
})
