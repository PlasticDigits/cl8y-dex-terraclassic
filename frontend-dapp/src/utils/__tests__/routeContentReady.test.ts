import { describe, expect, it, vi } from 'vitest'
import { dispatchRouteContentReady, ROUTE_CONTENT_READY_EVENT } from '../routeContentReady'

describe('routeContentReady', () => {
  it('dispatches custom event with pathname detail', () => {
    const handler = vi.fn()
    window.addEventListener(ROUTE_CONTENT_READY_EVENT, handler)
    dispatchRouteContentReady('/trade/terra1abc')
    expect(handler).toHaveBeenCalledTimes(1)
    const event = handler.mock.calls[0][0] as CustomEvent<{ pathname: string }>
    expect(event.detail.pathname).toBe('/trade/terra1abc')
    window.removeEventListener(ROUTE_CONTENT_READY_EVENT, handler)
  })
})
