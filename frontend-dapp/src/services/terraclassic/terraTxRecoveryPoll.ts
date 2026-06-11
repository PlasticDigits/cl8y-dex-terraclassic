import { getTx } from '@goblinhunt/cosmes/client'
import type { CosmosTxV1beta1GetTxResponse as GetTxResponse } from '@goblinhunt/cosmes/protobufs'
import { TERRA_TX_POST_SIGN_NOT_FOUND_MESSAGE } from '@/utils/terraTxTimeout'

const RECOVERY_POLL_INTERVAL_MS = 2_000

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

/**
 * Poll LCD for a signed tx hash until the on-chain deadline or inclusion (GitLab #359).
 */
export async function pollTxUntilRecoveryDeadline(
  rpcEndpoint: string,
  txHash: string,
  deadlineUnixSec: number
): Promise<Required<GetTxResponse>> {
  while (true) {
    const nowSec = Math.floor(Date.now() / 1000)

    try {
      const res = await getTx(rpcEndpoint, { hash: txHash })
      if (res.tx && res.txResponse) {
        return res as Required<GetTxResponse>
      }
    } catch {
      // Tx not indexed yet — keep polling through the deadline window.
    }

    if (nowSec >= deadlineUnixSec) {
      throw new Error(TERRA_TX_POST_SIGN_NOT_FOUND_MESSAGE)
    }

    const remainingMs = Math.max(0, (deadlineUnixSec - nowSec) * 1000)
    await sleep(Math.min(RECOVERY_POLL_INTERVAL_MS, remainingMs))
  }
}
