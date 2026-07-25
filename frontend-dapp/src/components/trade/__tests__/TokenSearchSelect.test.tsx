import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { TokenSearchSelect } from '@/components/trade/TokenSearchSelect'
import { renderWithProviders } from '@/test-utils'
import { TOKEN_SEARCH_DEBOUNCE_MS } from '@/utils/tokenSearchQuery'

const EMBER_ADDR = 'terra1ember00000000000000000000000000000001'
const CORAL_ADDR = 'terra1coral00000000000000000000000000000002'
const JADE_ADDR = 'terra1jade000000000000000000000000000000003'
const TOKEN_CACHE_KEY = 'cl8y-dex-token-info'

const tokens = [EMBER_ADDR, CORAL_ADDR, JADE_ADDR, 'uluna']

describe('TokenSearchSelect (GitLab #481)', () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    localStorage.setItem(
      TOKEN_CACHE_KEY,
      JSON.stringify({
        [EMBER_ADDR.toLowerCase()]: { symbol: 'EMBER', name: 'Ember' },
        [CORAL_ADDR.toLowerCase()]: { symbol: 'CORAL', name: 'Coral' },
        [JADE_ADDR.toLowerCase()]: { symbol: 'JADE', name: 'Jade' },
      })
    )
  })

  afterEach(() => {
    vi.useRealTimers()
    localStorage.removeItem(TOKEN_CACHE_KEY)
    vi.clearAllMocks()
  })

  it('lists allowed tokens on empty query and excludes the other leg', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    renderWithProviders(
      <TokenSearchSelect
        value={EMBER_ADDR}
        tokens={tokens}
        excludeToken={CORAL_ADDR}
        onChange={vi.fn()}
        aria-label="Select token you pay"
      />
    )

    const input = screen.getByRole('combobox', { name: 'Select token you pay' })
    await user.click(input)

    const listbox = await screen.findByRole('listbox', { name: 'Select token you pay' })
    await waitFor(() => {
      expect(within(listbox).getAllByRole('option').length).toBeGreaterThan(0)
    })
    expect(within(listbox).queryByTestId(`token-option-${CORAL_ADDR}`)).not.toBeInTheDocument()
    expect(within(listbox).getByTestId(`token-option-${EMBER_ADDR}`)).toBeInTheDocument()
    expect(within(listbox).getByTestId(`token-option-${JADE_ADDR}`)).toBeInTheDocument()
  })

  it('filters by symbol after debounce and selects via click', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    const onChange = vi.fn()
    renderWithProviders(
      <TokenSearchSelect
        value={EMBER_ADDR}
        tokens={tokens}
        excludeToken={CORAL_ADDR}
        onChange={onChange}
        aria-label="Select token you pay"
      />
    )

    const input = screen.getByRole('combobox', { name: 'Select token you pay' })
    await user.click(input)
    await user.clear(input)
    await user.type(input, 'jade')
    await vi.advanceTimersByTimeAsync(TOKEN_SEARCH_DEBOUNCE_MS + 50)

    const listbox = await screen.findByRole('listbox')
    await waitFor(() => {
      expect(within(listbox).getByTestId(`token-option-${JADE_ADDR}`)).toBeInTheDocument()
    })
    expect(within(listbox).queryByTestId(`token-option-${EMBER_ADDR}`)).not.toBeInTheDocument()

    await user.click(within(listbox).getByTestId(`token-option-${JADE_ADDR}`))
    expect(onChange).toHaveBeenCalledWith(JADE_ADDR)
  })

  it('shows empty state when no tokens match', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    renderWithProviders(
      <TokenSearchSelect value={EMBER_ADDR} tokens={tokens} onChange={vi.fn()} aria-label="Select token you pay" />
    )

    const input = screen.getByRole('combobox', { name: 'Select token you pay' })
    await user.click(input)
    await user.clear(input)
    await user.type(input, 'zzzzz')
    await vi.advanceTimersByTimeAsync(TOKEN_SEARCH_DEBOUNCE_MS + 50)

    await waitFor(() => {
      expect(screen.getByText(/No tokens match your search/i)).toBeInTheDocument()
    })
  })

  it('Enter commits the first search hit when query excludes the current token (#350)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    const onChange = vi.fn()
    renderWithProviders(
      <TokenSearchSelect value={EMBER_ADDR} tokens={tokens} onChange={onChange} aria-label="Select token you pay" />
    )

    const input = screen.getByRole('combobox', { name: 'Select token you pay' })
    await user.click(input)
    await user.clear(input)
    await user.type(input, 'jade')
    await vi.advanceTimersByTimeAsync(TOKEN_SEARCH_DEBOUNCE_MS + 50)

    await waitFor(() => {
      expect(screen.getByTestId(`token-option-${JADE_ADDR}`)).toBeInTheDocument()
    })

    await user.keyboard('{Enter}')
    expect(onChange).toHaveBeenCalledWith(JADE_ADDR)
    expect(onChange).not.toHaveBeenCalledWith(EMBER_ADDR)
  })

  it('Escape closes without changing value', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    const onChange = vi.fn()
    renderWithProviders(
      <TokenSearchSelect value={EMBER_ADDR} tokens={tokens} onChange={onChange} aria-label="Select token you pay" />
    )

    const input = screen.getByRole('combobox', { name: 'Select token you pay' })
    await user.click(input)
    await user.type(input, 'ja')
    await user.keyboard('{Escape}')

    expect(onChange).not.toHaveBeenCalled()
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
  })

  it('disabled / empty tokens shows disabled combobox', () => {
    renderWithProviders(
      <TokenSearchSelect value="" tokens={[]} onChange={vi.fn()} aria-label="Select token you pay" disabled />
    )
    expect(screen.getByRole('combobox', { name: 'Select token you pay' })).toBeDisabled()
  })

  it('keeps leading logo and selected label on open without editing (GitLab #498)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    renderWithProviders(
      <TokenSearchSelect value={EMBER_ADDR} tokens={tokens} onChange={vi.fn()} aria-label="Select token you pay" />
    )

    const input = screen.getByRole('combobox', { name: 'Select token you pay' })
    expect(screen.getByTestId('token-search-leading-logo')).toBeInTheDocument()
    expect(input).toHaveClass('token-select-trigger--with-leading-logo')
    expect(input).toHaveValue('EMBER')

    await user.click(input)
    await screen.findByRole('listbox', { name: 'Select token you pay' })

    expect(screen.getByTestId('token-search-leading-logo')).toBeInTheDocument()
    expect(input).toHaveClass('token-select-trigger--with-leading-logo')
    expect(input).toHaveValue('EMBER')
  })

  it('renders XSS-looking metadata as plain text only', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    const evil = 'terra1evil000000000000000000000000000000001'
    localStorage.setItem(
      TOKEN_CACHE_KEY,
      JSON.stringify({
        [evil.toLowerCase()]: { symbol: '<script>alert(1)</script>', name: '<img src=x onerror=alert(1)>' },
      })
    )

    renderWithProviders(
      <TokenSearchSelect value={evil} tokens={[evil]} onChange={vi.fn()} aria-label="Select token you pay" />
    )

    const input = screen.getByRole('combobox', { name: 'Select token you pay' })
    await user.click(input)
    const listbox = await screen.findByRole('listbox')
    const option = within(listbox).getByRole('option')
    expect(option.textContent).toContain('<script>alert(1)</script>')
    expect(option.querySelector('script')).toBeNull()
  })

  it('cannot select excluded token via search tricks', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    const onChange = vi.fn()
    renderWithProviders(
      <TokenSearchSelect
        value={EMBER_ADDR}
        tokens={tokens}
        excludeToken={JADE_ADDR}
        onChange={onChange}
        aria-label="Select token you pay"
      />
    )

    const input = screen.getByRole('combobox', { name: 'Select token you pay' })
    await user.click(input)
    await user.clear(input)
    await user.type(input, 'jade')
    await vi.advanceTimersByTimeAsync(TOKEN_SEARCH_DEBOUNCE_MS + 50)

    await waitFor(() => {
      expect(screen.getByText(/No tokens match your search/i)).toBeInTheDocument()
    })
    expect(screen.queryByTestId(`token-option-${JADE_ADDR}`)).not.toBeInTheDocument()
    expect(onChange).not.toHaveBeenCalled()
  })
})
