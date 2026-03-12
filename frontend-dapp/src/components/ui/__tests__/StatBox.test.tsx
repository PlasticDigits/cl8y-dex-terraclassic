import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { StatBox } from '../StatBox'

describe('StatBox', () => {
  it('renders label and value', () => {
    render(<StatBox label="Volume" value="1.5K" />)
    expect(screen.getByText('Volume')).toBeInTheDocument()
    expect(screen.getByText('1.5K')).toBeInTheDocument()
  })

  it('shows skeleton when loading', () => {
    const { container } = render(<StatBox label="Volume" value="1.5K" loading />)
    expect(screen.queryByText('1.5K')).not.toBeInTheDocument()
    const skeleton = container.querySelector('.animate-pulse')
    expect(skeleton).toBeInTheDocument()
  })

  it('applies custom color', () => {
    render(<StatBox label="PnL" value="+5%" color="var(--color-positive)" />)
    const valueEl = screen.getByText('+5%')
    expect(valueEl).toHaveStyle({ color: 'var(--color-positive)' })
  })
})
