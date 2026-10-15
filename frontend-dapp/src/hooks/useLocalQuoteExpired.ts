import { useEffect, useState } from 'react'
import { flushSync } from 'react-dom'
import { isExpiredLocalQuote, localQuoteExpiryMs } from '@/utils/localQuoteFreshness'

/** One bounded timer per original snapshot, shared by pay and receive sizing.
 * Early timer delivery rechecks the original wall-clock deadline; it never
 * renews observation metadata or starts a quote/provider request. */
export function useLocalQuoteExpired(data: unknown): boolean {
  const [, refreshExpiry] = useState(0)
  const expiry = localQuoteExpiryMs(data)
  useEffect(() => {
    if (expiry === undefined || !Number.isFinite(expiry) || expiry <= Date.now()) return
    let timer: number
    // Browser timeout overflow turns oversized delays into immediate callbacks.
    // Clamp only the timer wait, never the original observation deadline.
    const delay = (remaining: number) => Math.min(2_147_483_647, Math.max(1, remaining))
    const expire = () => {
      const remaining = expiry - Date.now()
      if (remaining > 0) timer = window.setTimeout(expire, delay(remaining))
      // A concurrent React update can leave the old executable amount in the DOM
      // after this original deadline until another scheduler task commits it.
      // Revoke display and payload in this timer task, before another paint/read.
      else flushSync(() => refreshExpiry((value) => value + 1))
    }
    timer = window.setTimeout(expire, delay(expiry - Date.now()))
    return () => window.clearTimeout(timer)
  }, [expiry])
  return isExpiredLocalQuote(data)
}
