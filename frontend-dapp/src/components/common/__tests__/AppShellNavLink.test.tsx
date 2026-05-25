import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom'
import { describe, expect, it, vi } from 'vitest'
import { AppShellNavLink } from '../AppShellNavLink'

vi.mock('@/lib/sounds', () => ({
  sounds: { playButtonPress: vi.fn() },
}))

function LocationEcho() {
  const { pathname } = useLocation()
  return <div data-testid="pathname">{pathname}</div>
}

describe('AppShellNavLink', () => {
  it('navigates on plain left-click (GitLab #182)', async () => {
    const user = userEvent.setup()
    render(
      <MemoryRouter initialEntries={['/']}>
        <AppShellNavLink item={{ path: '/pool', label: 'Pool' }} className={() => 'app-nav-link'} />
        <Routes>
          <Route path="/" element={<LocationEcho />} />
          <Route path="/pool" element={<LocationEcho />} />
        </Routes>
      </MemoryRouter>
    )

    await user.click(screen.getByRole('link', { name: 'Pool' }))
    expect(screen.getByTestId('pathname')).toHaveTextContent('/pool')
  })

  it('does not intercept modified clicks', () => {
    render(
      <MemoryRouter initialEntries={['/']}>
        <AppShellNavLink item={{ path: '/pool', label: 'Pool' }} className={() => 'app-nav-link'} />
        <Routes>
          <Route path="/" element={<LocationEcho />} />
          <Route path="/pool" element={<LocationEcho />} />
        </Routes>
      </MemoryRouter>
    )

    fireEvent.click(screen.getByRole('link', { name: 'Pool' }), { metaKey: true })
    expect(screen.getByTestId('pathname')).toHaveTextContent('/')
  })
})
