import { describe, expect, it, vi } from 'vitest'
import { useState } from 'react'
import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MenuSelect } from '@/components/ui/MenuSelect'

const OPTIONS = [
  { value: 'a', label: 'Alpha pair' },
  { value: 'b', label: 'Beta pair' },
  { value: 'c', label: 'Gamma pair' },
  { value: 'd', label: 'Delta pair' },
]

function ControlledMenuSelect({ initial = 'a' }: { initial?: string }) {
  const [value, setValue] = useState(initial)
  return <MenuSelect id="test-menu" value={value} options={OPTIONS} onChange={setValue} aria-label="Test pair" />
}

describe('MenuSelect keyboard (GitLab #244)', () => {
  it('opens with ArrowDown and sets aria-activedescendant on the listbox', async () => {
    const user = userEvent.setup()
    render(<ControlledMenuSelect />)

    const trigger = screen.getByRole('button', { name: 'Test pair' })
    trigger.focus()
    await user.keyboard('{ArrowDown}')

    const listbox = screen.getByRole('listbox', { name: 'Test pair' })
    expect(listbox).toBeInTheDocument()
    expect(listbox).toHaveAttribute('aria-activedescendant')
    expect(listbox.getAttribute('aria-activedescendant')).toMatch(/-option-0$/)
  })

  it('selects the third option after ArrowDown x3 and Enter', async () => {
    const user = userEvent.setup()
    render(<ControlledMenuSelect />)

    const trigger = screen.getByRole('button', { name: 'Test pair' })
    trigger.focus()
    await user.keyboard('{ArrowDown}{ArrowDown}{ArrowDown}{Enter}')

    expect(screen.getByRole('button', { name: 'Test pair' })).toHaveTextContent('Gamma pair')
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
  })

  it('jumps via typeahead prefix on the option label', async () => {
    const user = userEvent.setup()
    render(<ControlledMenuSelect />)

    const trigger = screen.getByRole('button', { name: 'Test pair' })
    trigger.focus()
    await user.keyboard('de')

    const listbox = screen.getByRole('listbox', { name: 'Test pair' })
    const activeId = listbox.getAttribute('aria-activedescendant')
    expect(activeId).toBeTruthy()
    expect(document.getElementById(activeId!)).toHaveTextContent('Delta pair')
  })

  it('accumulates rapid typeahead from a closed trigger before open state commits', () => {
    render(<ControlledMenuSelect />)

    const trigger = screen.getByRole('button', { name: 'Test pair' })
    trigger.focus()
    fireEvent.keyDown(trigger, { key: 'd' })
    fireEvent.keyDown(trigger, { key: 'e' })

    const listbox = screen.getByRole('listbox', { name: 'Test pair' })
    const activeId = listbox.getAttribute('aria-activedescendant')
    expect(document.getElementById(activeId!)).toHaveTextContent('Delta pair')
  })

  it('closes on Escape and restores focus to the trigger', async () => {
    const user = userEvent.setup()
    render(<ControlledMenuSelect />)

    const trigger = screen.getByRole('button', { name: 'Test pair' })
    trigger.focus()
    await user.keyboard('{ArrowDown}')
    expect(screen.getByRole('listbox')).toBeInTheDocument()

    await user.keyboard('{Escape}')
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
    expect(trigger).toHaveFocus()
  })

  it('still selects via click without regression', async () => {
    const user = userEvent.setup()
    render(<ControlledMenuSelect />)

    await user.click(screen.getByRole('button', { name: 'Test pair' }))
    await user.click(screen.getByRole('option', { name: 'Beta pair' }))

    expect(screen.getByRole('button', { name: 'Test pair' })).toHaveTextContent('Beta pair')
  })

  it('marks aria-selected on the current value', async () => {
    const user = userEvent.setup()
    render(<ControlledMenuSelect initial="b" />)

    await user.click(screen.getByRole('button', { name: 'Test pair' }))
    expect(screen.getByRole('option', { name: 'Alpha pair' })).toHaveAttribute('aria-selected', 'false')
    expect(screen.getByRole('option', { name: 'Beta pair' })).toHaveAttribute('aria-selected', 'true')
  })

  it('does not spam onChange on rapid Enter key repeat', async () => {
    const user = userEvent.setup()
    const onChange = vi.fn()

    function Harness() {
      const [value, setValue] = useState('')
      return (
        <MenuSelect
          value={value}
          options={OPTIONS}
          onChange={(v) => {
            onChange(v)
            setValue(v)
          }}
          aria-label="Test pair"
        />
      )
    }

    render(<Harness />)

    const trigger = screen.getByRole('button', { name: 'Test pair' })
    trigger.focus()
    await user.keyboard('{ArrowDown}{Enter}{Enter}{Enter}')

    expect(onChange).toHaveBeenCalledTimes(1)
    expect(onChange).toHaveBeenCalledWith('a')
  })
})
