import { render, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useEffect } from 'react'
import { MemoryRouter, NavLink, Route, Routes, useLocation } from 'react-router-dom'
import { describe, expect, it, vi } from 'vitest'
import { ROUTE_CONTENT_READY_FAILSAFE_MS } from '../routeContentReadyConstants'
import { RouteContentReadyProvider } from '../RouteContentReadyContext'
import { useMarkRouteContentReady } from '../useMarkRouteContentReady'

function RouteMarker() {
  const { pathname } = useLocation()
  const mark = useMarkRouteContentReady()
  useEffect(() => {
    mark(pathname)
  }, [pathname, mark])
  return null
}

function Shell({ onReady }: { onReady: (ready: boolean) => void }) {
  return (
    <RouteContentReadyProvider onReadyChange={onReady}>
      <nav>
        <NavLink to="/swap">Swap</NavLink>
        <NavLink to="/trade">Trade</NavLink>
      </nav>
      <Routes>
        <Route path="/swap" element={<RouteMarker />} />
        <Route path="/trade" element={<RouteMarker />} />
      </Routes>
    </RouteContentReadyProvider>
  )
}

describe('RouteContentReadyProvider', () => {
  it('exposes a 12s failsafe for stuck routes (GitLab #179)', () => {
    expect(ROUTE_CONTENT_READY_FAILSAFE_MS).toBe(12_000)
  })

  it('becomes ready after the route marker effect runs', async () => {
    const onReady = vi.fn()
    render(
      <MemoryRouter initialEntries={['/swap']}>
        <Shell onReady={onReady} />
      </MemoryRouter>
    )
    await waitFor(() => {
      expect(onReady).toHaveBeenCalledWith(true)
    })
  })

  it('reports not ready on the first render after navigation (GitLab #138)', async () => {
    const onReady = vi.fn()
    const user = userEvent.setup()
    render(
      <MemoryRouter initialEntries={['/swap']}>
        <Shell onReady={onReady} />
      </MemoryRouter>
    )
    await waitFor(() => expect(onReady).toHaveBeenCalledWith(true))
    onReady.mockClear()

    await user.click(document.querySelector('a[href="/trade"]')!)

    expect(onReady.mock.calls.some(([ready]) => ready === false)).toBe(true)
    await waitFor(() => expect(onReady).toHaveBeenCalledWith(true))
  })
})
