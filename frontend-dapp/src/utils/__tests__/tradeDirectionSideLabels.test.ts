import { describe, expect, it } from 'vitest'
import { tradeDirectionSideLabels } from '@/utils/tradeDirectionSideLabels'

describe('tradeDirectionSideLabels', () => {
  it('labels bid as Buy base and ask as Sell base (GitLab #412)', () => {
    const { bidLabel, askLabel } = tradeDirectionSideLabels('EMBER')
    expect(bidLabel).toBe('Buy EMBER')
    expect(askLabel).toBe('Sell EMBER')
  })

  it('uses base symbol only so buttons match ticket Buy/Sell heading', () => {
    const { bidLabel, askLabel } = tradeDirectionSideLabels('BTC')
    expect(bidLabel).toMatch(/^Buy /)
    expect(askLabel).toMatch(/^Sell /)
    expect(bidLabel).toContain('BTC')
    expect(askLabel).toContain('BTC')
    expect(askLabel).not.toContain('USDT')
  })
})
