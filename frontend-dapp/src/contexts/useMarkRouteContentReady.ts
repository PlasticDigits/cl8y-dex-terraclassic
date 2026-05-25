import { useContext } from 'react'
import { RouteContentReadyContext } from './routeContentReadyContextState'

export function useMarkRouteContentReady(): (pathname: string) => void {
  const ctx = useContext(RouteContentReadyContext)
  if (!ctx) {
    throw new Error('useMarkRouteContentReady must be used within RouteContentReadyProvider')
  }
  return ctx.markRouteContentReady
}
