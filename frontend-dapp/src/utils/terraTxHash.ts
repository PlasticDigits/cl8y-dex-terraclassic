import type { CosmosTxV1beta1TxRaw as TxRaw } from '@goblinhunt/cosmes/protobufs'

/** SHA-256 of protobuf-encoded `TxRaw` — matches LCD / RPC tx hash (GitLab #359). */
export async function txHashFromTxRaw(txRaw: TxRaw): Promise<string> {
  const bytes = txRaw.toBinary()
  const digest = await crypto.subtle.digest('SHA-256', bytes)
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('')
    .toUpperCase()
}
