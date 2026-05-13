import { describe, it, expect } from 'vitest'
import { TRADE_INDEXER_OUTAGE_BANNER_LEAD, TRADE_INDEXER_OUTAGE_BANNER_TAIL } from '../indexerTradeOutageCopy'

describe('indexerTradeOutageCopy', () => {
  it('does not claim order book still works via chain only (GitLab #164)', () => {
    const combined = `${TRADE_INDEXER_OUTAGE_BANNER_LEAD} ${TRADE_INDEXER_OUTAGE_BANNER_TAIL}`.toLowerCase()
    expect(combined).not.toContain('still use chain')
    expect(combined).not.toContain('where applicable')
  })

  it('mentions limit reference pool fallback when tape is missing (GitLab #166)', () => {
    expect(TRADE_INDEXER_OUTAGE_BANNER_TAIL).toMatch(/#166|166/i)
    expect(TRADE_INDEXER_OUTAGE_BANNER_TAIL).toMatch(/pool|LCD/i)
  })

  it('names degraded trade surfaces (indexer dependency)', () => {
    expect(TRADE_INDEXER_OUTAGE_BANNER_LEAD).toMatch(/order book|chart|tape|swap|limit/i)
    expect(TRADE_INDEXER_OUTAGE_BANNER_TAIL).toMatch(/unavailable|degraded/i)
  })
})
