import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import LegalFooterNotice from '@/components/legal/LegalFooterNotice'
import { SECURITY_REPORT_ISSUE_URL, USER_INCIDENT_FAQ_HREF } from '@/components/legal/legalCopy'
import { SECURITY_POSTURE_DOC_URL } from '@/utils/constants'

describe('LegalFooterNotice', () => {
  it('links to the public security posture doc in a new tab', () => {
    render(<LegalFooterNotice />)

    const link = screen.getByTestId('security-posture-doc-link')
    expect(link).toHaveAttribute('href', SECURITY_POSTURE_DOC_URL)
    expect(link).toHaveAttribute('target', '_blank')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
    expect(link).toHaveTextContent('Security')
    expect(SECURITY_POSTURE_DOC_URL).toMatch(/security-posture\.md$/)
  })

  it('links to the user incident FAQ (GitLab #390)', () => {
    render(<LegalFooterNotice />)

    const link = screen.getByTestId('user-incident-faq-link')
    expect(link).toHaveAttribute('href', USER_INCIDENT_FAQ_HREF)
    expect(link).toHaveTextContent('Incidents')
    expect(link).toHaveAttribute('target', '_blank')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
  })

  it('links suspicious-activity reports to the GitLab security template', () => {
    render(<LegalFooterNotice />)

    const link = screen.getByRole('link', { name: /^report$/i })
    expect(link).toHaveAttribute('href', SECURITY_REPORT_ISSUE_URL)
    expect(link).toHaveAttribute('target', '_blank')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
  })
})
