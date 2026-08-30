import type { ReactElement } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router-dom'
import { RouteContentReadyProvider } from '@/contexts/RouteContentReadyContext'
import { STALE_CHUNK_RELOAD_STORAGE_KEY, staleChunkReloadIo } from '@/utils/chunkLoadError'
import { LazyRoute } from '../LazyRoute'

const CHUNK_ERR = new TypeError(
  'Failed to fetch dynamically imported module: https://dex.cl8y.com/assets/PoolPage-BrgV5Tp1.js'
)

function renderLazyRoute(ui: ReactElement) {
  return render(
    <MemoryRouter>
      <RouteContentReadyProvider onReadyChange={() => {}}>{ui}</RouteContentReadyProvider>
    </MemoryRouter>
  )
}

describe('LazyRoute', () => {
  beforeEach(() => {
    sessionStorage.clear()
    vi.stubGlobal('navigator', { ...navigator, onLine: true })
    vi.spyOn(staleChunkReloadIo, 'reloadDocument').mockImplementation(() => {})
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
    sessionStorage.clear()
  })

  it('re-runs dynamic import when Try Again is clicked after offline chunk failure (GitLab #172)', async () => {
    vi.stubGlobal('navigator', { ...navigator, onLine: false })
    const user = userEvent.setup()
    let importCalls = 0

    renderLazyRoute(
      <LazyRoute
        loader={async () => {
          importCalls += 1
          if (importCalls < 2) {
            throw new TypeError(
              'Failed to fetch dynamically imported module: http://localhost:3000/src/pages/MockPage.tsx'
            )
          }
          return {
            default: function MockPage() {
              return <p>Loaded page</p>
            },
          }
        }}
      />
    )

    await waitFor(() => {
      expect(screen.getByTestId('route-error-boundary')).toBeInTheDocument()
    })
    expect(screen.getByText(/could not load/i)).toBeInTheDocument()
    expect(importCalls).toBe(1)
    expect(screen.queryByTestId('stale-chunk-updating')).not.toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /try again/i }))

    await waitFor(() => {
      expect(screen.getByText('Loaded page')).toBeInTheDocument()
    })
    expect(importCalls).toBe(2)
  })

  it('online first chunk error one-shot reloads instead of flashing Page unavailable (GitLab #706)', async () => {
    renderLazyRoute(<LazyRoute loader={() => Promise.reject(CHUNK_ERR)} />)

    await waitFor(() => {
      expect(staleChunkReloadIo.reloadDocument).toHaveBeenCalledTimes(1)
    })
    expect(sessionStorage.getItem(STALE_CHUNK_RELOAD_STORAGE_KEY)).toBe('1')
    expect(screen.getByTestId('stale-chunk-updating')).toBeInTheDocument()
    expect(screen.queryByTestId('route-error-boundary')).not.toBeInTheDocument()
  })

  it('after helper already fired, UI shows Reload app; click reloads (not only loadAttempt++)', async () => {
    sessionStorage.setItem(STALE_CHUNK_RELOAD_STORAGE_KEY, '1')
    const user = userEvent.setup()
    let importCalls = 0

    renderLazyRoute(
      <LazyRoute
        loader={async () => {
          importCalls += 1
          throw CHUNK_ERR
        }}
      />
    )

    await waitFor(() => {
      expect(screen.getByTestId('route-error-boundary')).toBeInTheDocument()
    })
    expect(screen.getByTestId('route-error-reload-app')).toBeInTheDocument()
    expect(staleChunkReloadIo.reloadDocument).not.toHaveBeenCalled()
    expect(importCalls).toBe(1)

    await user.click(screen.getByTestId('route-error-reload-app'))
    expect(staleChunkReloadIo.reloadDocument).toHaveBeenCalledTimes(1)
    expect(importCalls).toBe(1)
  })

  it('successful lazy mount clears the stale-chunk reload guard', async () => {
    sessionStorage.setItem(STALE_CHUNK_RELOAD_STORAGE_KEY, '1')
    renderLazyRoute(
      <LazyRoute
        loader={async () => ({
          default: function MockPage() {
            return <p>Loaded page</p>
          },
        })}
      />
    )
    await waitFor(() => {
      expect(screen.getByText('Loaded page')).toBeInTheDocument()
    })
    expect(sessionStorage.getItem(STALE_CHUNK_RELOAD_STORAGE_KEY)).toBeNull()
  })

  it('non-chunk render errors do not auto-reload or show Reload app', async () => {
    renderLazyRoute(
      <LazyRoute
        loader={async () => ({
          default: function BoomPage(): never {
            throw new Error('Max spread assertion')
          },
        })}
      />
    )
    await waitFor(() => {
      expect(screen.getByTestId('route-error-boundary')).toBeInTheDocument()
    })
    expect(staleChunkReloadIo.reloadDocument).not.toHaveBeenCalled()
    expect(screen.queryByTestId('route-error-reload-app')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: /try again/i })).toBeInTheDocument()
  })

  it('renders custom fallback while lazy page loads', () => {
    renderLazyRoute(
      <LazyRoute fallback={<p data-testid="custom-fallback">Custom loading</p>} loader={() => new Promise(() => {})} />
    )
    expect(screen.getByTestId('custom-fallback')).toBeInTheDocument()
  })

  it('shows route-scoped headline for chunk errors when auto-reload is blocked', async () => {
    sessionStorage.setItem(STALE_CHUNK_RELOAD_STORAGE_KEY, '1')
    renderLazyRoute(
      <LazyRoute
        loader={() =>
          Promise.reject(new TypeError('Failed to fetch dynamically imported module: http://localhost/chunk.js'))
        }
      />
    )
    await waitFor(() => {
      expect(screen.getByText('Page unavailable')).toBeInTheDocument()
    })
  })

  it('header-path: route fallback stays inside layout testid (wallet chrome not wiped)', async () => {
    sessionStorage.setItem(STALE_CHUNK_RELOAD_STORAGE_KEY, '1')
    renderLazyRoute(<LazyRoute loader={() => Promise.reject(CHUNK_ERR)} />)
    await waitFor(() => {
      expect(screen.getByTestId('route-error-boundary')).toBeInTheDocument()
    })
    expect(screen.queryByTestId('app-error-boundary')).not.toBeInTheDocument()
  })
})
