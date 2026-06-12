import type { ConnectedWallet } from '@goblinhunt/cosmes/wallet'

export const TERRA_TX_RECOVERY_UNKNOWN_MESSAGE = 'Broadcast status unknown — the transaction may still confirm.'

export const TERRA_TX_RECOVERY_EXPIRED_MESSAGE =
  'Transaction was not confirmed before the deadline. You may retry if the swap did not execute.'

const RECOVERY_POLL_INTERVAL_MS = 2_000

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

/**
 * Polls LCD/RPC for a signed tx hash until inclusion or the msg deadline (GitLab #359).
 * Does not re-broadcast — safe when the original RPC submit may still be in flight.
 */
export async function pollTerraTxRecovery(
  wallet: ConnectedWallet,
  txHash: string,
  deadlineUnix: number
): Promise<void> {
  while (Math.floor(Date.now() / 1000) < deadlineUnix) {
    try {
      const { txResponse } = await wallet.pollTx(txHash, { maxAttempts: 1, intervalSeconds: 1 })
      if (txResponse.code !== 0) {
        const raw = txResponse.rawLog || txResponse.logs?.[0]?.log || `Transaction failed with code ${txResponse.code}`
        throw new Error(raw)
      }
      return
    } catch (error: unknown) {
      if (error instanceof Error && !/not found|tx not found/i.test(error.message)) {
        throw error
      }
    }
    await sleep(RECOVERY_POLL_INTERVAL_MS)
  }

  throw new Error(TERRA_TX_RECOVERY_EXPIRED_MESSAGE)
}
