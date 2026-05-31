import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { TokenSelect } from '@/components/ui/TokenSelect'

vi.mock('@/hooks/useTokenDisplayInfo', () => ({
  useTokenDisplayInfo: (info: { native_token?: { denom: string }; token?: { contract_addr: string } }) => {
    const id = info.native_token?.denom ?? info.token?.contract_addr ?? ''
    const labels: Record<string, string> = {
      uluna: 'LUNC',
      uusd: 'USTC',
      terra1cl8y: 'CL8Y',
    }
    const symbol = labels[id] ?? id
    return { displayLabel: symbol, symbol, addressForBlockie: undefined, logoURI: undefined }
  },
}))

const TOKENS = ['uluna', 'uusd', 'terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3']

describe('TokenSelect keyboard (GitLab #244)', () => {
  it('typeahead matches token symbol prefix case-insensitively', async () => {
    const user = userEvent.setup()
    render(<TokenSelect value="uluna" tokens={TOKENS} onChange={() => {}} aria-label="Pay token" />)

    const trigger = screen.getByRole('button', { name: 'Pay token' })
    trigger.focus()
    await user.keyboard('us')

    const listbox = screen.getByRole('listbox', { name: 'Pay token' })
    const activeId = listbox.getAttribute('aria-activedescendant')
    expect(document.getElementById(activeId!)).toHaveTextContent('USTC')
  })

  it('selects third token via ArrowDown x3 and Enter', async () => {
    const user = userEvent.setup()
    const onChange = vi.fn()
    render(<TokenSelect value="uluna" tokens={TOKENS} onChange={onChange} aria-label="Pay token" />)

    const trigger = screen.getByRole('button', { name: 'Pay token' })
    trigger.focus()
    await user.keyboard('{ArrowDown}{ArrowDown}{ArrowDown}{Enter}')

    expect(onChange).toHaveBeenCalledWith('terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3')
  })
})
