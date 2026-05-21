import { describe, expect, it } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { LazyRoute } from '../LazyRoute'

describe('LazyRoute', () => {
  it('re-runs dynamic import when Try Again is clicked after chunk failure', async () => {
    const user = userEvent.setup()
    let importCalls = 0

    render(
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

    await user.click(screen.getByRole('button', { name: /try again/i }))

    await waitFor(() => {
      expect(screen.getByText('Loaded page')).toBeInTheDocument()
    })
    expect(importCalls).toBe(2)
  })

  it('shows route-scoped headline for chunk errors', async () => {
    render(
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
})
