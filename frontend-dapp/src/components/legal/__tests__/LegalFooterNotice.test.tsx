import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import LegalFooterNotice from '@/components/legal/LegalFooterNotice'
import { SECURITY_REPORT_ISSUE_URL } from '@/components/legal/legalCopy'

describe('LegalFooterNotice', () => {
  it('links suspicious-activity reports to the GitLab security template', () => {
    render(<LegalFooterNotice />)

    const link = screen.getByRole('link', { name: /report suspicious activity/i })
    expect(link).toHaveAttribute('href', SECURITY_REPORT_ISSUE_URL)
    expect(link).toHaveAttribute('target', '_blank')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
  })
})
