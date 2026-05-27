import { describe, it, expect } from 'vitest'
import {
  TRADE_INDEXER_OUTAGE_BANNER_LEAD,
  TRADE_INDEXER_OUTAGE_BANNER_TAIL,
  TRADE_INDEXER_OUTAGE_BANNER_TITLE,
  TRADE_PANEL_BOOK_UNAVAILABLE,
  TRADE_PANEL_CHART_UNAVAILABLE,
  TRADE_PANEL_TAPE_UNAVAILABLE,
} from '../indexerTradeOutageCopy'

describe('indexerTradeOutageCopy', () => {
  const combined = `${TRADE_INDEXER_OUTAGE_BANNER_TITLE} ${TRADE_INDEXER_OUTAGE_BANNER_LEAD} ${TRADE_INDEXER_OUTAGE_BANNER_TAIL}`

  it('does not claim order book still works via chain only (GitLab #164)', () => {
    expect(combined.toLowerCase()).not.toContain('still use chain')
    expect(combined.toLowerCase()).not.toContain('where applicable')
  })

  it('mentions limit reference pool fallback when tape is missing (GitLab #166)', () => {
    expect(TRADE_INDEXER_OUTAGE_BANNER_TAIL).toMatch(/pool|wallet|buy-below|sell-above/i)
    expect(TRADE_INDEXER_OUTAGE_BANNER_TAIL).not.toMatch(/\[GitLab|\]\(/)
  })

  it('names degraded trade surfaces without exposing env URLs (GitLab #174)', () => {
    expect(TRADE_INDEXER_OUTAGE_BANNER_TITLE).toMatch(/market data service/i)
    expect(TRADE_INDEXER_OUTAGE_BANNER_LEAD).toMatch(/order book|chart|tape|swap|limit/i)
    expect(TRADE_INDEXER_OUTAGE_BANNER_LEAD).toMatch(/limited|recovers/i)
    expect(combined).not.toMatch(/VITE_INDEXER_URL|127\.0\.0\.1/i)
    expect(combined.toLowerCase()).not.toContain('indexer unavailable at')
  })

  it('panel copy explains each trade surface during outage (GitLab #165)', () => {
    for (const line of [TRADE_PANEL_BOOK_UNAVAILABLE, TRADE_PANEL_TAPE_UNAVAILABLE, TRADE_PANEL_CHART_UNAVAILABLE]) {
      expect(line).toMatch(/market data service/i)
      expect(line.toLowerCase()).not.toContain('indexer')
      expect(line.toLowerCase()).not.toContain('lcd')
    }
  })
})
