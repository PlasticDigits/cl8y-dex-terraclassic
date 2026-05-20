import type { ReactNode } from 'react'
import type { UseQueryResult } from '@tanstack/react-query'
import { RetryError, Skeleton } from '@/components/ui'
import { getErrorMessage } from '@/utils/humanizeUserFacingError'
import { isLcdConnectivityError, LCD_CONNECTIVITY_OUTAGE_MESSAGE } from '@/utils/lcdConnectivity'

type GateQuery = Pick<UseQueryResult<unknown>, 'isLoading' | 'isError' | 'error' | 'refetch'>

export interface LcdQueryGateProps {
  query: GateQuery
  /** Shown while the query is loading (first fetch). */
  loadingFallback?: ReactNode
  /** When true, render nothing instead of children while loading (for inline sections). */
  hideWhileLoading?: boolean
  children: ReactNode
}

const defaultLoading = <Skeleton height="2.5rem" width="100%" />

/**
 * Blocks children until an LCD-backed query succeeds; surfaces RetryError on RPC/LCD failure ([GitLab #171](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/171)).
 */
export function LcdQueryGate({
  query,
  loadingFallback = defaultLoading,
  hideWhileLoading = false,
  children,
}: LcdQueryGateProps) {
  if (query.isLoading) {
    if (hideWhileLoading) return null
    return <>{loadingFallback}</>
  }

  if (query.isError) {
    const message = isLcdConnectivityError(query.error) ? LCD_CONNECTIVITY_OUTAGE_MESSAGE : getErrorMessage(query.error)
    return <RetryError message={message} onRetry={() => void query.refetch()} />
  }

  return <>{children}</>
}
