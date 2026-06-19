import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import LegalFooterNotice from '@/components/legal/LegalFooterNotice'
import { SECURITY_POSTURE_DOC_URL } from '@/utils/constants'

describe('LegalFooterNotice', () => {
  it('links to the public security posture doc in a new tab', () => {
    render(<LegalFooterNotice />)

    const link = screen.getByTestId('security-posture-doc-link')
    expect(link).toHaveAttribute('href', SECURITY_POSTURE_DOC_URL)
    expect(link).toHaveAttribute('target', '_blank')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
    expect(link).toHaveTextContent('Security and audit docs')
    expect(SECURITY_POSTURE_DOC_URL).toMatch(/security-posture\.md$/)
  })
})
