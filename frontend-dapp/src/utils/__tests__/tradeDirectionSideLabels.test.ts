import { describe, expect, it } from 'vitest'
import { tradeDirectionSideLabels } from '@/utils/tradeDirectionSideLabels'

describe('tradeDirectionSideLabels', () => {
  it('labels bid with base and ask with quote (GitLab #300)', () => {
    const { bidLabel, askLabel } = tradeDirectionSideLabels('EMBER', 'CORAL')
    expect(bidLabel).toBe('Buy EMBER')
    expect(askLabel).toBe('Buy CORAL')
  })
})
