/** Dispatched when a lazy route page mounts so shell chrome can defer low-priority paint (GitLab #179). */
export const ROUTE_CONTENT_READY_EVENT = 'cl8y:route-content-ready'

export function dispatchRouteContentReady(pathname: string) {
  if (typeof window === 'undefined') return
  window.dispatchEvent(new CustomEvent(ROUTE_CONTENT_READY_EVENT, { detail: { pathname } }))
}
