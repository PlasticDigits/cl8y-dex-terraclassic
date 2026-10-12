import { CosmosTxV1beta1AuthInfo as AuthInfo } from '@goblinhunt/cosmes/protobufs'
import { RpcClient } from '@goblinhunt/cosmes/client'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { assertSignedOfflineSwapFee, SIGNED_OFFLINE_SWAP_FEE_MISMATCH } from '../signedOfflineSwapFee'
import { buildTerraClassicFee } from '../terraGas'
import { broadcastTerraExecuteContracts } from '../terraBroadcast'
import { signTerraTxRaw } from '../terraWalletSignTxRaw'
import { installSwapGasReaderForTests } from '../swapAutoGas'

vi.mock('../terraWalletSignTxRaw', async (importActual) => ({
  ...(await importActual<typeof import('../terraWalletSignTxRaw')>()),
  signTerraTxRaw: vi.fn(),
}))

function signed(gas: bigint, amount = ((gas * 28_325n + 999n) / 1000n).toString(), denom = 'uluna') {
  return new AuthInfo({ fee: { gasLimit: gas, amount: [{ denom, amount }] } }).toBinary()
}
const requested = buildTerraClassicFee(3_200_000)
afterEach(() => {
  vi.restoreAllMocks()
  vi.clearAllMocks()
  installSwapGasReaderForTests(null)
})

describe('signed Fee inspection scoped to offline reviewed swaps (#1480)', () => {
  it('accepts the reviewed gas and compares exact uluna', () => {
    const gas = 3_200_000n
    const receipt = assertSignedOfflineSwapFee(signed(gas), requested)
    expect(receipt.requestedGas).toBe(3_200_000n)
    expect(receipt.signedGas).toBe(gas)
    expect(receipt.signedUluna).toBe((gas * 28_325n + 999n) / 1000n)
  })
  it.each([
    signed(3_199_999n, '999999999'), // Higher fee amount does not fix lower Fee.gas.
    signed(3_200_000n, '1'),
    signed(3_300_000n),
    signed(3_200_000n, '90640001'),
    signed(15_000_001n),
    signed(3_200_000n, '90640000', 'uusd'),
    new Uint8Array(),
    new Uint8Array([255]),
    new AuthInfo({ fee: { ...requested, payer: 'terra1other' } }).toBinary(),
    new AuthInfo({ fee: { ...requested, granter: 'terra1other' } }).toBinary(),
  ])('rejects lower/corrupt/over-limit signed Fee without reflecting wallet fields', (bytes) => {
    expect(() => assertSignedOfflineSwapFee(bytes, requested)).toThrow(SIGNED_OFFLINE_SWAP_FEE_MISMATCH)
  })
  it('checks actual split signing result before broadcast on columbus-5 with no simulation or resend', async () => {
    const rpc = vi.spyOn(RpcClient, 'broadcastTx').mockResolvedValue({} as never)
    const gasReader = vi.fn(async () => 1_000_000)
    installSwapGasReaderForTests(gasReader)
    const wallet = { chainId: 'columbus-5', rpc: 'https://rpc.invalid', ext: { signDirect: vi.fn() }, pollTx: vi.fn() }
    vi.mocked(signTerraTxRaw).mockResolvedValue({
      txRaw: { authInfoBytes: signed(3_199_999n) },
      txHash: 'HASH',
      sequence: 1n,
    } as never)
    await expect(
      broadcastTerraExecuteContracts(
        wallet as never,
        'terra1actor',
        [
          {
            contract: 'terra1token',
            msg: {
              send: {
                contract: 'terra1router',
                amount: '1000000',
                msg: btoa(
                  JSON.stringify({
                    execute_swap_operations: {
                      operations: [{ terra_swap: {} }],
                      minimum_receive: '950000',
                    },
                  })
                ),
              },
            },
          },
        ],
        { gasEstimation: 'offline' }
      )
    ).rejects.toThrow(SIGNED_OFFLINE_SWAP_FEE_MISMATCH)
    expect(signTerraTxRaw).toHaveBeenCalledTimes(1)
    expect(rpc).not.toHaveBeenCalled()
    expect(wallet.pollTx).not.toHaveBeenCalled()
    expect(gasReader).not.toHaveBeenCalled()
  })
  it('does not broadcast a protected direct wrapper when original quote expires during signing', async () => {
    const rpc = vi.spyOn(RpcClient, 'broadcastTx').mockResolvedValue({} as never)
    const expiry = Date.now() + 10_000
    const wallet = { chainId: 'columbus-5', rpc: 'https://rpc.invalid', ext: { signDirect: vi.fn() }, pollTx: vi.fn() }
    vi.mocked(signTerraTxRaw).mockImplementation(async () => {
      vi.spyOn(Date, 'now').mockReturnValue(expiry)
      return { txRaw: { authInfoBytes: signed(800_000n) }, txHash: 'HASH', sequence: 1n } as never
    })
    await expect(
      broadcastTerraExecuteContracts(
        wallet as never,
        'terra1actor',
        [
          {
            contract: 'terra1treasury',
            msg: { wrap_deposit_with_minimum: { minimum_receive: '1' } },
            coins: [{ denom: 'uusd', amount: '1' }],
          },
        ],
        {
          gasEstimation: 'offline',
          reviewedQuoteExpiresAtMs: expiry,
        }
      )
    ).rejects.toThrow('Quote expired. Refresh the quote.')
    expect(signTerraTxRaw).toHaveBeenCalledTimes(1)
    expect(rpc).not.toHaveBeenCalled()
    expect(wallet.pollTx).not.toHaveBeenCalled()
  })
})
