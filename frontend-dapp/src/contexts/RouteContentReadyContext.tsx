import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'
import { useLocation } from 'react-router-dom'
import { ROUTE_CONTENT_READY_FAILSAFE_MS } from './routeContentReadyConstants'
import { RouteContentReadyContext } from './routeContentReadyContextState'

/**
 * Tracks which pathname has mounted route content so Layout can defer legal footer LCP (#179)
 * while keeping NFA visible immediately after navigation (#138).
 *
 * `routeContentReady` is true only when `readyForPath === pathname`, so a stale ready
 * path never satisfies a new route before `RouteContentReadyMarker` runs (child effects
 * run before parent effects; window events were dropped — see GitLab #138). Avoid
 * render-phase `setState` here — it can interfere with React Router navigation (#182).
 */
export function RouteContentReadyProvider({
  children,
  onReadyChange,
}: {
  children: ReactNode
  onReadyChange: (ready: boolean) => void
}) {
  const { pathname } = useLocation()
  const [readyForPath, setReadyForPath] = useState<string | null>(null)

  /** Stale `readyForPath` never satisfies a new pathname (no render-phase setState — GitLab #182). */
  const routeContentReady = readyForPath === pathname

  const markRouteContentReady = useCallback((path: string) => {
    setReadyForPath(path)
  }, [])

  useEffect(() => {
    onReadyChange(routeContentReady)
  }, [onReadyChange, routeContentReady])

  useEffect(() => {
    const failsafe = window.setTimeout(() => markRouteContentReady(pathname), ROUTE_CONTENT_READY_FAILSAFE_MS)
    return () => window.clearTimeout(failsafe)
  }, [pathname, markRouteContentReady])

  const value = useMemo(() => ({ markRouteContentReady }), [markRouteContentReady])

  return <RouteContentReadyContext.Provider value={value}>{children}</RouteContentReadyContext.Provider>
}
