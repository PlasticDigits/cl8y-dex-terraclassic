import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { LcdQueryGate } from '../LcdQueryGate'

describe('LcdQueryGate', () => {
  it('shows loading fallback while query is loading', () => {
    render(
      <LcdQueryGate
        query={{ isLoading: true, isError: false, error: null, refetch: vi.fn() }}
        loadingFallback={<p>Loading…</p>}
      >
        <p>Content</p>
      </LcdQueryGate>
    )
    expect(screen.getByText('Loading…')).toBeInTheDocument()
    expect(screen.queryByText('Content')).not.toBeInTheDocument()
  })

  it('shows retry on LCD connectivity error', async () => {
    const refetch = vi.fn()
    render(
      <LcdQueryGate
        query={{
          isLoading: false,
          isError: true,
          error: new Error('LCD request timed out after 10000ms'),
          refetch,
        }}
      >
        <p>Content</p>
      </LcdQueryGate>
    )
    expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument()
    expect(screen.getByText(/could not connect to the network/i)).toBeInTheDocument()
    expect(screen.getByText(/on-chain funds are safe/i)).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /retry/i }))
    expect(refetch).toHaveBeenCalled()
  })

  it('renders children when query succeeded', () => {
    render(
      <LcdQueryGate query={{ isLoading: false, isError: false, error: null, refetch: vi.fn() }}>
        <p>Content</p>
      </LcdQueryGate>
    )
    expect(screen.getByText('Content')).toBeInTheDocument()
  })
})
