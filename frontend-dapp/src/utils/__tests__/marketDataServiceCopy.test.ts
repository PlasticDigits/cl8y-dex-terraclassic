import { describe, it, expect } from 'vitest'
import {
  CHARTS_MARKET_DATA_OUTAGE_LEAD,
  MARKET_DATA_SERVICE_OUTAGE_TITLE,
  POOL_MARKET_DATA_OUTAGE_LEAD,
  PROTOCOL_MARKET_DATA_OUTAGE_LEAD,
  TRADER_MARKET_DATA_OUTAGE_LEAD,
} from '../marketDataServiceCopy'

describe('marketDataServiceCopy', () => {
  const allLeads = [
    CHARTS_MARKET_DATA_OUTAGE_LEAD,
    TRADER_MARKET_DATA_OUTAGE_LEAD,
    POOL_MARKET_DATA_OUTAGE_LEAD,
    PROTOCOL_MARKET_DATA_OUTAGE_LEAD,
  ]

  it('uses market data service wording without env URLs (GitLab #174, #215)', () => {
    expect(MARKET_DATA_SERVICE_OUTAGE_TITLE).toMatch(/market data service/i)
    for (const lead of allLeads) {
      expect(lead).toMatch(/limited|recovers/i)
      expect(lead).not.toMatch(/VITE_INDEXER_URL|127\.0\.0\.1/i)
      expect(lead.toLowerCase()).not.toContain('indexer unavailable')
    }
  })
})
