import { useEffect } from 'react'
import { useLocation } from 'react-router-dom'
import { useMarkRouteContentReady } from '@/contexts/useMarkRouteContentReady'

/** Signals when route page content mounts so Layout can defer legal footer LCP (GitLab #179). */
export function RouteContentReadyMarker({ children }: { children: React.ReactNode }) {
  const { pathname } = useLocation()
  const markRouteContentReady = useMarkRouteContentReady()

  useEffect(() => {
    markRouteContentReady(pathname)
  }, [pathname, markRouteContentReady])

  return <>{children}</>
}
