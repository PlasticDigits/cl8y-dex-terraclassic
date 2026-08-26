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

  it('flat variant omits card-glass and renders inline Δ% (GitLab #652)', () => {
    const { container } = render(
      <StatBox variant="flat" label="Last 24h vol" value="$1.2K" delta="+50%" deltaLabel="24h" deltaTestId="vol-chg" />
    )
    expect(container.querySelector('.card-glass')).toBeNull()
    expect(screen.getByTestId('vol-chg')).toHaveTextContent('+50%')
    expect(screen.getByTestId('vol-chg')).toHaveTextContent('24h')
  })

  it('groups Δ% with the headline instead of justify-between (GitLab #667)', () => {
    const { container } = render(
      <StatBox
        variant="flat"
        label="Last 24h vol"
        value="$3.7K"
        delta="+155.5%"
        deltaLabel="24h"
        deltaTestId="vol-chg"
      />
    )
    const row = container.querySelector('.stat-value-row')
    expect(row).toBeTruthy()
    expect(row?.className).not.toMatch(/justify-between/)
    expect(container.querySelector('.stat-delta-cluster')).toBeTruthy()
    expect(screen.getByTestId('vol-chg').closest('.stat-value-row')).toBe(row)
  })

  it('keeps two liquidity chips in one cluster after the headline (GitLab #667)', () => {
    const { container } = render(
      <StatBox
        variant="flat"
        label="Total liquidity"
        value="$61K"
        deltas={[
          { value: '+19.74%', label: '24h', testId: 'liq-24h' },
          { value: '—', label: '30d', testId: 'liq-30d' },
        ]}
      />
    )
    const cluster = container.querySelector('.stat-delta-cluster')
    expect(cluster).toBeTruthy()
    expect(cluster).toContainElement(screen.getByTestId('liq-24h'))
    expect(cluster).toContainElement(screen.getByTestId('liq-30d'))
    expect(screen.getByTestId('liq-30d')).toHaveTextContent('—')
  })

  it('value-only tiles keep a value row without a delta cluster', () => {
    const { container } = render(<StatBox variant="flat" label="Tokens" value="14" />)
    expect(container.querySelector('.stat-value-row')).toBeTruthy()
    expect(container.querySelector('.stat-delta-cluster')).toBeNull()
    expect(screen.getByText('14')).toBeInTheDocument()
  })

  it('puts title on the card and label and aria-label on the value (GitLab #576)', () => {
    render(
      <StatBox
        label="Last 24h Vol (USD)"
        value="$1.2K"
        title="Priced swaps in the last 24 hours, not a midnight reset."
      />
    )
    const card = screen.getByText('Last 24h Vol (USD)').closest('[title]')
    expect(card).toHaveAttribute('title', 'Priced swaps in the last 24 hours, not a midnight reset.')
    expect(screen.getByText('Last 24h Vol (USD)')).toHaveAttribute(
      'title',
      'Priced swaps in the last 24 hours, not a midnight reset.'
    )
    expect(screen.getByLabelText(/last 24 hours, not a midnight reset/i)).toHaveTextContent('$1.2K')
  })

  it('default variant keeps card-glass (GitLab #653)', () => {
    const { container } = render(<StatBox label="Volume" value="1.5K" />)
    const root = container.firstElementChild
    expect(root?.className).toMatch(/card-glass/)
    expect(root?.className).not.toMatch(/stat-flat/)
  })

  it('flat variant has no card-glass (GitLab #653)', () => {
    const { container } = render(<StatBox variant="flat" label="Volume" value="1.5K" />)
    const root = container.firstElementChild
    expect(root?.className).toMatch(/stat-flat/)
    expect(root?.className).not.toMatch(/card-glass/)
  })

  it('flat keeps title and value aria-label (GitLab #653)', () => {
    render(
      <StatBox
        variant="flat"
        label="Last 24h Vol (USD)"
        value="$1.2K"
        title="Priced swaps in the last 24 hours, not a midnight reset."
      />
    )
    expect(screen.getByText('Last 24h Vol (USD)')).toHaveAttribute(
      'title',
      'Priced swaps in the last 24 hours, not a midnight reset.'
    )
    expect(screen.getByLabelText(/last 24 hours, not a midnight reset/i)).toHaveTextContent('$1.2K')
  })
})
