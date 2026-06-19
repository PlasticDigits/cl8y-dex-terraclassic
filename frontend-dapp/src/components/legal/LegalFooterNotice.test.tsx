import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import LegalFooterNotice from './LegalFooterNotice'
import { USER_INCIDENT_FAQ_HREF, USER_INCIDENT_FAQ_LABEL } from './legalCopy'

describe('LegalFooterNotice', () => {
  it('links to the user incident FAQ (GitLab #390)', () => {
    render(<LegalFooterNotice />)

    const link = screen.getByTestId('user-incident-faq-link')
    expect(link).toHaveAttribute('href', USER_INCIDENT_FAQ_HREF)
    expect(link).toHaveTextContent(USER_INCIDENT_FAQ_LABEL)
    expect(link).toHaveAttribute('target', '_blank')
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
  })
})
