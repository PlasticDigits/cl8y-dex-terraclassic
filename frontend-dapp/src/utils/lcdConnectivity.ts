/**
 * LCD / RPC reachability helpers for React Query error UX and auto-recovery ([GitLab #171](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/171)).
 */
import { tryHumanizeFetchLikeMessage } from '@/utils/humanizeOffChainError'
import { getErrorMessage } from '@/utils/humanizeUserFacingError'
import { TERRA_LCD_URL } from '@/utils/constants'

/** Retail copy when the Terra LCD endpoint is unreachable (W11-C2). */
export const LCD_CONNECTIVITY_OUTAGE_MESSAGE = 'Could not connect to the network. Check your connection or try again.'

export const LCD_CONNECTIVITY_RECOVERY_POLL_MS = 5_000

const LCD_PROBE_TIMEOUT_MS = 5_000

const LCD_ERROR_PATTERNS = [
  /lcd request timed out/i,
  /lcd fetch failed/i,
  /failed to fetch contract info/i,
  /query failed:\s*5\d{2}/i,
  /balance query failed/i,
  /network request failed/i,
  /failed to fetch/i,
  /networkerror when attempting to fetch/i,
  /load failed/i,
  /net::err_/i,
  /aborterror/i,
  /the operation was aborted/i,
  /signal is aborted/i,
] as const

/** True when an error likely indicates LCD / chain RPC transport failure (not business logic). */
export function isLcdConnectivityError(error: unknown): boolean {
  const msg = getErrorMessage(error)
  if (!msg) return false
  if (LCD_ERROR_PATTERNS.some((re) => re.test(msg))) return true
  return tryHumanizeFetchLikeMessage(msg) != null
}

/** Lightweight GET against the configured LCD — used for recovery polling, not for contract reads. */
export async function probeLcdReachability(): Promise<boolean> {
  const url = `${TERRA_LCD_URL}/cosmos/base/tendermint/v1beta1/node_info`
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), LCD_PROBE_TIMEOUT_MS)
  try {
    const response = await fetch(url, { signal: controller.signal, method: 'GET' })
    return response.ok
  } catch {
    return false
  } finally {
    clearTimeout(timer)
  }
}
