import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import Cl8yProductLinks from '@/components/common/Cl8yProductLinks'
import { CL8Y_PRODUCT_BRIDGE_HREF, CL8Y_PRODUCT_HOME_HREF } from '@/utils/cl8yProductLinks'

describe('Cl8yProductLinks (GitLab #663)', () => {
  it('renders Homepage and Bridge with pinned href, new-tab, and rel', () => {
    render(<Cl8yProductLinks />)

    const nav = screen.getByRole('navigation', { name: 'CL8Y products' })
    expect(nav).toBeInTheDocument()

    const home = screen.getByTestId('footer-product-home')
    expect(home).toHaveTextContent('Homepage')
    expect(home).toHaveAttribute('href', CL8Y_PRODUCT_HOME_HREF)
    expect(home).toHaveAttribute('target', '_blank')
    expect(home).toHaveAttribute('rel', 'noopener noreferrer')
    expect(home.tagName).toBe('A')

    const bridge = screen.getByTestId('footer-product-bridge')
    expect(bridge).toHaveTextContent('Bridge')
    expect(bridge).toHaveAttribute('href', CL8Y_PRODUCT_BRIDGE_HREF)
    expect(bridge).toHaveAttribute('target', '_blank')
    expect(bridge).toHaveAttribute('rel', 'noopener noreferrer')

    expect(screen.getByRole('link', { name: 'Homepage' })).toBe(home)
    expect(screen.getByRole('link', { name: 'Bridge' })).toBe(bridge)
  })

  it('does not iframe or use phishing-adjacent labels', () => {
    const { container } = render(<Cl8yProductLinks />)
    expect(container.querySelector('iframe')).toBeNull()
    expect(screen.queryByRole('link', { name: /^security$/i })).not.toBeInTheDocument()
    expect(screen.queryByRole('link', { name: /^report$/i })).not.toBeInTheDocument()
    expect(screen.queryByRole('link', { name: /connect wallet/i })).not.toBeInTheDocument()
  })
})
