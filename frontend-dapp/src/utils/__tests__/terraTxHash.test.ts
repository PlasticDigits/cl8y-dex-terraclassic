import { CosmosTxV1beta1TxRaw as ProtoTxRaw } from '@goblinhunt/cosmes/protobufs'
import { describe, expect, it } from 'vitest'
import { txHashFromTxRaw } from '../terraTxHash'

describe('txHashFromTxRaw (GitLab #359)', () => {
  it('returns uppercase hex SHA-256 of TxRaw bytes', async () => {
    const txRaw = new ProtoTxRaw({
      bodyBytes: new Uint8Array([1, 2, 3]),
      authInfoBytes: new Uint8Array([4, 5]),
      signatures: [new Uint8Array([6])],
    })

    const hash = await txHashFromTxRaw(txRaw)
    expect(hash).toMatch(/^[0-9A-F]{64}$/)
    expect(hash).toBe(hash.toUpperCase())
  })
})
