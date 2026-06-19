import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import LegalFooterNotice from '@/components/legal/LegalFooterNotice'
import {
  SECURITY_REPORT_ISSUE_URL,
  USER_INCIDENT_FAQ_HREF,
  USER_INCIDENT_FAQ_LABEL,
} from '@/components/legal/legalCopy'

describe('LegalFooterNotice', () => {
  it('links to the user incident FAQ (GitLab #390)', () => {
    render(<LegalFooterNotice />)

    const link = screen.getByTestId('user-incident-faq-link')
    expect(link).toHaveAttribute('href', USER_INCIDENT_FAQ_HREF)
    expect(link).toHaveTextContent(USER_INCIDENT_FAQ_LABEL)
    expect(link).toHaveAttribute('target', '_blank')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
  })

  it('links suspicious-activity reports to the GitLab security template', () => {
    render(<LegalFooterNotice />)

    const link = screen.getByRole('link', { name: /report suspicious activity/i })
    expect(link).toHaveAttribute('href', SECURITY_REPORT_ISSUE_URL)
    expect(link).toHaveAttribute('target', '_blank')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
  })
})
