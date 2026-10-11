import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { QuoteDataLoadingProgress } from './QuoteDataLoadingProgress'

describe('indeterminate price publication progress', () => {
  it('announces a busy data wait without inventing completion percentages or an ETA', () => {
    render(<QuoteDataLoadingProgress testId="pricing-wait" />)
    expect(screen.getByRole('status')).toHaveAttribute('aria-live', 'polite')
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-busy', 'true')
    expect(screen.getByRole('progressbar', { name: 'Loading price' })).not.toHaveAttribute('value')
    expect(screen.getByRole('progressbar')).not.toHaveAttribute('aria-valuenow')
    expect(screen.getByTestId('pricing-wait')).toHaveTextContent('Loading price…')
    expect(screen.getByTestId('pricing-wait').textContent).not.toMatch(/%|ETA|seconds/i)
  })
})
