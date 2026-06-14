import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import { TokenLogo } from '@/components/ui/TokenLogo'

describe('TokenLogo allowlist', () => {
  it('falls back to blockie for disallowed logo hosts (GitLab #378)', () => {
    render(<TokenLogo addressForBlockie="terra1abc" logoURI="https://evil.example/logo.png" size={24} />)
    expect(screen.queryByRole('img')).not.toBeInTheDocument()
  })

  it('renders allowlisted https logo URIs', () => {
    const uri = 'https://assets.coingecko.com/coins/images/1/large/bitcoin.png'
    const { container } = render(<TokenLogo addressForBlockie="terra1abc" logoURI={uri} size={24} />)
    expect(container.querySelector('img')).toHaveAttribute('src', uri)
  })
})
