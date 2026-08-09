import { useEffect, useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { queryRateLimit } from '@/services/terraclassic/wrapMapper'
import { WRAP_MAPPER_CONTRACT_ADDRESS } from '@/utils/constants'
import { formatTokenAmountAbbrev } from '@/utils/formatAmount'
import { deriveWrapRateLimitStatus, formatWrapRateLimitCountdown } from '@/utils/wrapRateLimit'

const DECIMALS = 6

export type WrapRateLimitStatusProps = {
  /** Native denom keyed on wrap-mapper (`uluna` / `uusd`). */
  denom: string | null | undefined
  /** Display symbol for amounts (LUNC / USTC). */
  symbol: string
  enabled?: boolean
  testId?: string
  className?: string
}

/**
 * Live wrap-mapper rate-limit availability + reset countdown (GitLab #502 / #507).
 */
export function WrapRateLimitStatus({
  denom,
  symbol,
  enabled = true,
  testId = 'wrap-rate-limit-status',
  className = '',
}: WrapRateLimitStatusProps) {
  const [nowSec, setNowSec] = useState(() => Math.floor(Date.now() / 1000))

  const query = useQuery({
    queryKey: ['wrapRateLimitStatus', denom],
    queryFn: () => queryRateLimit(denom!),
    enabled: !!WRAP_MAPPER_CONTRACT_ADDRESS && !!denom && enabled,
    staleTime: 15_000,
    refetchInterval: 30_000,
  })

  const status = useMemo(() => deriveWrapRateLimitStatus(query.data, nowSec), [query.data, nowSec])

  useEffect(() => {
    if (!status?.windowActive || status.secondsUntilReset == null || status.secondsUntilReset <= 0) {
      return
    }
    const id = window.setInterval(() => setNowSec(Math.floor(Date.now() / 1000)), 1000)
    return () => window.clearInterval(id)
  }, [status?.windowActive, status?.secondsUntilReset])

  if (!denom || !enabled) return null
  if (query.isLoading && !query.data) {
    return (
      <div className={`text-xs ${className}`} style={{ color: 'var(--ink-dim)' }} data-testid={testId}>
        Loading wrap limit…
      </div>
    )
  }
  if (query.isError) {
    return (
      <div className={`text-xs ${className}`} style={{ color: 'var(--ink-dim)' }} data-testid={testId}>
        Wrap limit unavailable
      </div>
    )
  }
  if (!status) return null

  const remainingLabel = formatTokenAmountAbbrev(status.remainingRaw.toString(), DECIMALS, 4)
  const maxLabel = formatTokenAmountAbbrev(status.maxRaw.toString(), DECIMALS, 4)
  const countdown = formatWrapRateLimitCountdown(status.secondsUntilReset)
  const exhausted = status.remainingRaw === 0n

  return (
    <div
      className={`text-xs space-y-0.5 ${className}`}
      style={{ color: exhausted ? 'var(--danger, #c44)' : 'var(--ink-dim)' }}
      data-testid={testId}
      role="status"
    >
      <p data-testid={`${testId}-available`}>
        <span className="uppercase tracking-wide font-medium" style={{ color: 'var(--ink-subtle)' }}>
          Wrap limit
        </span>{' '}
        <span className="font-mono tabular-nums">
          {remainingLabel} / {maxLabel} {symbol}
        </span>{' '}
        available
      </p>
      {status.windowActive && countdown && (
        <p data-testid={`${testId}-reset`}>
          Resets in <span className="font-mono tabular-nums">{countdown}</span>
        </p>
      )}
      {status.windowExpired && <p data-testid={`${testId}-reset`}>Limit window ended — next wrap opens a new window</p>}
      {!status.windowActive && !status.windowExpired && status.secondsUntilReset == null && (
        <p data-testid={`${testId}-reset`}>No wraps in the current window yet</p>
      )}
    </div>
  )
}
