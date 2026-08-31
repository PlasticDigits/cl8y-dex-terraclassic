import { Navigate, useLocation } from 'react-router-dom'

/**
 * `/swap` and `/swap/` alias → `/` while preserving search + hash (GitLab #711).
 * Do not mount SwapPage here — Swap tab `end: true` stays active only on `/`.
 * Redirect target is always a location object (never concatenated user strings).
 */
export function SwapAliasRedirect() {
  const location = useLocation()
  return <Navigate to={{ pathname: '/', search: location.search, hash: location.hash }} replace />
}
