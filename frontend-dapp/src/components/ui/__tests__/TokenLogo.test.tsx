import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { TokenLogo } from '@/components/ui/TokenLogo'

describe('TokenLogo allowlist', () => {
  it('falls back to blockie for untrusted logo hosts (GitLab #378)', () => {
    render(<TokenLogo addressForBlockie="terra1abc" logoURI="https://evil.example/logo.png" size={24} />)
    expect(screen.queryByRole('img')).not.toBeInTheDocument()
  })

  it('renders allowlisted remote logos', () => {
    const { container } = render(
      <TokenLogo
        addressForBlockie="terra1abc"
        logoURI="https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/LUNC.png"
        size={24}
      />
    )
    const img = container.querySelector('img')
    expect(img).toHaveAttribute(
      'src',
      'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/LUNC.png'
    )
  })
})
