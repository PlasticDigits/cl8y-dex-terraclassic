import type { ReactElement } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ErrorBoundary } from '../ErrorBoundary'
import { STALE_CHUNK_RELOAD_STORAGE_KEY, staleChunkReloadIo } from '@/utils/chunkLoadError'

function Boom({ message }: { message: string }): ReactElement {
  throw new Error(message)
}

describe('ErrorBoundary', () => {
  beforeEach(() => {
    sessionStorage.clear()
    vi.stubGlobal('navigator', { ...navigator, onLine: true })
    vi.spyOn(console, 'error').mockImplementation(() => {})
    vi.spyOn(staleChunkReloadIo, 'reloadDocument').mockImplementation(() => {})
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    sessionStorage.clear()
    vi.restoreAllMocks()
  })

  it('app-level chunk path still offers Reload App (GitLab #172)', async () => {
    sessionStorage.setItem(STALE_CHUNK_RELOAD_STORAGE_KEY, '1')
    const user = userEvent.setup()
    render(
      <ErrorBoundary>
        <Boom message="Failed to fetch dynamically imported module: https://dex.cl8y.com/assets/PoolPage-BrgV5Tp1.js" />
      </ErrorBoundary>
    )
    await waitFor(() => {
      expect(screen.getByTestId('app-error-boundary')).toBeInTheDocument()
    })
    await user.click(screen.getByRole('button', { name: /reload app/i }))
    expect(staleChunkReloadIo.reloadDocument).toHaveBeenCalledTimes(1)
  })

  it('app-level non-chunk errors do not call location.reload', async () => {
    render(
      <ErrorBoundary>
        <Boom message="Max spread assertion" />
      </ErrorBoundary>
    )
    await waitFor(() => {
      expect(screen.getByTestId('app-error-boundary')).toBeInTheDocument()
    })
    expect(staleChunkReloadIo.reloadDocument).not.toHaveBeenCalled()
    expect(screen.getByRole('button', { name: /reload app/i })).toBeInTheDocument()
  })
})
