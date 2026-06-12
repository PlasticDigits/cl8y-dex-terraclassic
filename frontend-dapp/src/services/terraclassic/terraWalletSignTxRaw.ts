import type { CosmosTxV1beta1TxRaw as TxRaw } from 'cosmes/protobufs'
import { sha256 } from '@noble/hashes/sha256'
import { bytesToHex } from '@noble/hashes/utils'
import * as CosmesClient from '@goblinhunt/cosmes/client'

type RpcBroadcastFn = (endpoint: string, txRaw: TxRaw) => Promise<string>

/** Tendermint tx hash: uppercase hex of sha256(TxRaw bytes). */
export function txHashFromTxRaw(txRaw: TxRaw): string {
  return bytesToHex(sha256(txRaw.toBinary())).toUpperCase()
}

export type SignedTxHashCapture = {
  readonly signedTxHash: string | null
  restore: () => void
}

function rpcBroadcastFn(): RpcBroadcastFn | null {
  const rpc = (CosmesClient as { RpcClient?: { broadcastTx?: RpcBroadcastFn } }).RpcClient
  if (!rpc || typeof rpc.broadcastTx !== 'function') return null
  return rpc.broadcastTx.bind(rpc)
}

/**
 * Intercepts cosmes `RpcClient.broadcastTx` to capture the post-sign hash before RPC returns.
 * Used when broadcast hangs or fails after signing (GitLab #359 / #368).
 * No-ops when RpcClient is unavailable (e.g. partial vitest mocks).
 */
export function installSignedTxHashCapture(): SignedTxHashCapture {
  const state = { signedTxHash: null as string | null }
  const original = rpcBroadcastFn()
  let restore: () => void = () => {}

  const rpc = (CosmesClient as { RpcClient?: { broadcastTx?: RpcBroadcastFn } }).RpcClient
  if (original && rpc) {
    rpc.broadcastTx = async (endpoint, txRaw) => {
      state.signedTxHash = txHashFromTxRaw(txRaw)
      return original(endpoint, txRaw)
    }
    restore = () => {
      rpc.broadcastTx = original
    }
  }

  return {
    get signedTxHash() {
      return state.signedTxHash
    },
    restore,
  }
}
