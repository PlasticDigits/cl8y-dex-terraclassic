import { useEffect } from 'react'
import { useLocation } from 'react-router-dom'
import { useMarkRouteContentReady } from '@/contexts/useMarkRouteContentReady'
import { clearStaleChunkReloadGuard } from '@/utils/chunkLoadError'

/** Signals when route page content mounts so Layout can defer legal footer LCP (GitLab #179). */
export function RouteContentReadyMarker({ children }: { children: React.ReactNode }) {
  const { pathname } = useLocation()
  const markRouteContentReady = useMarkRouteContentReady()

  useEffect(() => {
    markRouteContentReady(pathname)
    // Successful lazy mount: allow a later Coolify roll in this tab to one-shot reload (#706).
    clearStaleChunkReloadGuard()
  }, [pathname, markRouteContentReady])

  return <>{children}</>
}
