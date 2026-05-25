import { createContext } from 'react'

export type RouteContentReadyContextValue = {
  markRouteContentReady: (pathname: string) => void
}

export const RouteContentReadyContext = createContext<RouteContentReadyContextValue | null>(null)
