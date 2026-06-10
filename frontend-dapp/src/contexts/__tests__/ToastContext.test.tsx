import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ToastProvider } from '../ToastContext'
import { useToast } from '../toastContextState'

function ToastProbe() {
  const { pushToast } = useToast()
  return (
    <button type="button" onClick={() => pushToast('success', 'Limit order placed.')}>
      Fire toast
    </button>
  )
}

describe('ToastProvider (GitLab #351)', () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('renders a floating success toast and dismisses on click', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    render(
      <ToastProvider>
        <ToastProbe />
      </ToastProvider>
    )

    await user.click(screen.getByRole('button', { name: 'Fire toast' }))
    expect(screen.getByTestId('toast-viewport')).toBeInTheDocument()
    expect(screen.getByTestId('toast-success')).toHaveTextContent('Limit order placed.')

    await user.click(screen.getByRole('button', { name: 'Dismiss notification' }))
    expect(screen.queryByTestId('toast-success')).not.toBeInTheDocument()
  })

  it('auto-dismisses after timeout', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    render(
      <ToastProvider>
        <ToastProbe />
      </ToastProvider>
    )

    await user.click(screen.getByRole('button', { name: 'Fire toast' }))
    expect(screen.getByTestId('toast-success')).toBeInTheDocument()

    act(() => {
      vi.advanceTimersByTime(6000)
    })
    expect(screen.queryByTestId('toast-success')).not.toBeInTheDocument()
  })
})
