import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { TRADE_TAPE_EXPANDED_KEY, readTradePanelExpanded, writeTradePanelExpanded } from '../tradeWorkspacePanels'

describe('tradeWorkspacePanels', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  afterEach(() => {
    window.localStorage.clear()
  })

  it('defaults tape panel to collapsed on first visit', () => {
    expect(readTradePanelExpanded(TRADE_TAPE_EXPANDED_KEY, false)).toBe(false)
  })

  it('persists expanded state', () => {
    writeTradePanelExpanded(TRADE_TAPE_EXPANDED_KEY, true)
    expect(window.localStorage.getItem(TRADE_TAPE_EXPANDED_KEY)).toBe('1')
    expect(readTradePanelExpanded(TRADE_TAPE_EXPANDED_KEY, false)).toBe(true)
  })
})
