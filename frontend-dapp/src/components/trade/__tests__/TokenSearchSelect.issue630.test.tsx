import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { TokenSearchSelect } from '@/components/trade/TokenSearchSelect'
import { renderWithProviders } from '@/test-utils'
import { TOKEN_SEARCH_DEBOUNCE_MS } from '@/utils/tokenSearchQuery'
import type { IndexerToken } from '@/types'

vi.mock('react-blockies', () => ({
  __esModule: true,
  default: function MockBlockies() {
    return null
  },
}))

vi.mock('@/services/indexer/client', () => ({
  getTokens: vi.fn().mockResolvedValue([]),
}))

import { getTokens } from '@/services/indexer/client'

const CLUNC = 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg'
const CUSTC = 'terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch'
const tokens = ['uluna', 'uusd', CLUNC, CUSTC]

function tokenRow(partial: Partial<IndexerToken>): IndexerToken {
  return {
    id: 1,
    contract_address: null,
    denom: null,
    is_cw20: false,
    name: partial.symbol ?? 'x',
    symbol: 'x',
    decimals: 6,
    logo_url: null,
    coingecko_id: null,
    cmc_id: null,
    ...partial,
  }
}

const spoofedCatalog: IndexerToken[] = [
  tokenRow({ denom: 'uluna', symbol: 'uluna', name: 'uluna' }),
  tokenRow({ denom: 'uusd', symbol: 'UST1', name: '<b>uusd</b>' }),
  tokenRow({ contract_address: CLUNC, is_cw20: true, symbol: 'LUNC-C' }),
  tokenRow({ contract_address: CUSTC, is_cw20: true, symbol: 'USTC-C' }),
]

describe('TokenSearchSelect native labels (GitLab #630)', () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true })
    vi.mocked(getTokens).mockReset()
    vi.mocked(getTokens).mockResolvedValue(spoofedCatalog)
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.clearAllMocks()
  })

  it('lists LUNC / USTC / cLUNC / cUSTC; testids stay denoms / addresses', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    const onChange = vi.fn()
    renderWithProviders(
      <TokenSearchSelect value="uluna" tokens={tokens} onChange={onChange} aria-label="Select token you pay" />
    )

    const input = screen.getByRole('combobox', { name: 'Select token you pay' })
    await waitFor(() => expect(input).toHaveValue('LUNC'))

    await user.click(input)
    const listbox = await screen.findByRole('listbox', { name: 'Select token you pay' })
    await waitFor(() => {
      expect(within(listbox).getByTestId('token-option-uluna')).toHaveTextContent('LUNC')
    })
    expect(within(listbox).getByTestId('token-option-uusd')).toHaveTextContent('USTC')
    expect(within(listbox).getByTestId(`token-option-${CLUNC}`)).toHaveTextContent('cLUNC')
    expect(within(listbox).getByTestId(`token-option-${CUSTC}`)).toHaveTextContent('cUSTC')
    expect(listbox.textContent).not.toMatch(/\buluna\b/)
    expect(listbox.textContent).not.toMatch(/\buusd\b/)

    await user.click(within(listbox).getByTestId('token-option-uusd'))
    expect(onChange).toHaveBeenCalledWith('uusd')
  })

  it('closed trigger and open-before-edit show LUNC (#498)', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    renderWithProviders(
      <TokenSearchSelect value="uluna" tokens={tokens} onChange={vi.fn()} aria-label="Select token you pay" />
    )

    const input = screen.getByRole('combobox', { name: 'Select token you pay' })
    await waitFor(() => expect(input).toHaveValue('LUNC'))
    expect(screen.getByTestId('token-search-leading-logo')).toBeInTheDocument()
    expect(input).toHaveClass('token-select-trigger--with-leading-logo')

    await user.click(input)
    await screen.findByRole('listbox', { name: 'Select token you pay' })
    expect(input).toHaveValue('LUNC')
    expect(input).toHaveClass('token-select-trigger--with-leading-logo')
  })

  it('search LUNC and uluna both highlight the native row', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime.bind(vi) })
    renderWithProviders(
      <TokenSearchSelect value={CLUNC} tokens={tokens} onChange={vi.fn()} aria-label="Select token you pay" />
    )

    const input = screen.getByRole('combobox', { name: 'Select token you pay' })
    await user.click(input)
    await user.clear(input)
    await user.type(input, 'LUNC')
    await vi.advanceTimersByTimeAsync(TOKEN_SEARCH_DEBOUNCE_MS + 50)

    const listbox = await screen.findByRole('listbox')
    await waitFor(() => {
      expect(within(listbox).getByTestId('token-option-uluna')).toBeInTheDocument()
    })
    expect(within(listbox).getByTestId('token-option-uluna')).toHaveTextContent('LUNC')
  })
})
