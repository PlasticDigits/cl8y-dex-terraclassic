import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import { TradePageWorkspaceSkeleton } from '../TradePageWorkspaceSkeleton'

describe('TradePageWorkspaceSkeleton', () => {
  it('exposes status region for assistive tech', () => {
    render(<TradePageWorkspaceSkeleton />)
    expect(screen.getByRole('status', { name: /loading trade workspace/i })).toBeInTheDocument()
  })

  it('renders page chrome when includePageChrome is set', () => {
    const { container } = render(<TradePageWorkspaceSkeleton includePageChrome />)
    expect(container.querySelector('.shell-panel')).toBeTruthy()
  })

  it('renders both responsive skeleton regions', () => {
    render(<TradePageWorkspaceSkeleton />)
    expect(screen.getByTestId('trade-workspace-skeleton-sub-lg')).toBeInTheDocument()
    expect(screen.getByTestId('trade-workspace-skeleton-desktop')).toBeInTheDocument()
  })

  it('desktop skeleton keeps tape as an independent bottom row (GitLab #561)', () => {
    render(<TradePageWorkspaceSkeleton />)
    const desktop = screen.getByTestId('trade-workspace-skeleton-desktop')
    const tape = screen.getByTestId('trade-workspace-skeleton-desktop-tape')
    expect(desktop.contains(tape)).toBe(true)
    expect(desktop.style.gridTemplateColumns).toBe('')
    const chartish = desktop.querySelector('.shell-panel-strong')
    expect(chartish).toBeTruthy()
    expect(chartish?.contains(tape)).toBe(false)
  })
})
