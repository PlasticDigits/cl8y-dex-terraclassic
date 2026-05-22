import { useEffect } from 'react'
import { useLocation } from 'react-router-dom'
import { dispatchRouteContentReady } from '@/utils/routeContentReady'

/** Fires once when route page content mounts so Layout can defer legal footer LCP (GitLab #179). */
export function RouteContentReadyMarker({ children }: { children: React.ReactNode }) {
  const { pathname } = useLocation()

  useEffect(() => {
    dispatchRouteContentReady(pathname)
  }, [pathname])

  return <>{children}</>
}
