import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { TokenLogo } from './TokenLogo'

describe('TokenLogo allowlist (GitLab #378)', () => {
  it('renders blockie when logo host is not allowlisted', () => {
    render(
      <TokenLogo
        addressForBlockie="terra1abc1234567890123456789012345678901234567890"
        logoURI="https://evil.example/logo.png"
        size={24}
      />
    )
    expect(screen.queryByRole('img')).not.toBeInTheDocument()
  })

  it('renders img for allowlisted logo host', () => {
    const url = 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/raw/main/tokenlist/images/LUNC.png'
    const { container } = render(
      <TokenLogo addressForBlockie="terra1abc1234567890123456789012345678901234567890" logoURI={url} size={24} />
    )
    expect(container.querySelector('img')).toHaveAttribute('src', url)
  })
})
