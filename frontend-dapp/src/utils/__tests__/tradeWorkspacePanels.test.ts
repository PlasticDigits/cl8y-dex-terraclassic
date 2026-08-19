import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import {
  TRADE_BOOK_VISIBLE_KEY,
  TRADE_TAPE_EXPANDED_KEY,
  TRADE_TICKET_VISIBLE_KEY,
  readTradePanelExpanded,
  readTradePanelFlag,
  readTradePanelVisible,
  tradeDesktopChartGridColumn,
  tradeDesktopGridTemplateColumns,
  writeTradePanelExpanded,
  writeTradePanelVisible,
} from '../tradeWorkspacePanels'

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

  it('defaults book and ticket to visible; persists hide as 0 (GitLab #561)', () => {
    expect(readTradePanelVisible(TRADE_BOOK_VISIBLE_KEY, true)).toBe(true)
    expect(readTradePanelVisible(TRADE_TICKET_VISIBLE_KEY, true)).toBe(true)
    writeTradePanelVisible(TRADE_BOOK_VISIBLE_KEY, false)
    writeTradePanelVisible(TRADE_TICKET_VISIBLE_KEY, false)
    expect(window.localStorage.getItem(TRADE_BOOK_VISIBLE_KEY)).toBe('0')
    expect(readTradePanelVisible(TRADE_BOOK_VISIBLE_KEY, true)).toBe(false)
    expect(readTradePanelVisible(TRADE_TICKET_VISIBLE_KEY, true)).toBe(false)
  })

  it('ignores corrupt localStorage and falls back to defaults (GitLab #561 A1)', () => {
    window.localStorage.setItem(TRADE_BOOK_VISIBLE_KEY, 'true')
    window.localStorage.setItem(TRADE_TICKET_VISIBLE_KEY, '{"hidden":true}')
    window.localStorage.setItem(TRADE_TAPE_EXPANDED_KEY, '<script>')
    expect(readTradePanelVisible(TRADE_BOOK_VISIBLE_KEY, true)).toBe(true)
    expect(readTradePanelVisible(TRADE_TICKET_VISIBLE_KEY, true)).toBe(true)
    expect(readTradePanelExpanded(TRADE_TAPE_EXPANDED_KEY, false)).toBe(false)
    expect(readTradePanelFlag(TRADE_BOOK_VISIBLE_KEY, true)).toBe(true)
  })

  it('falls back when localStorage throws', () => {
    const proto = Object.getPrototypeOf(window.localStorage) as Storage
    const spy = vi.spyOn(proto, 'getItem').mockImplementation(() => {
      throw new Error('quota')
    })
    expect(readTradePanelVisible(TRADE_TICKET_VISIBLE_KEY, true)).toBe(true)
    spy.mockRestore()
  })

  it('gives the chart the vacated column when a side panel hides', () => {
    expect(tradeDesktopGridTemplateColumns(true, true)).toBe('minmax(13rem, 1fr) minmax(0, 2.2fr) minmax(13rem, 1fr)')
    expect(tradeDesktopGridTemplateColumns(false, true)).toBe('minmax(0, 3.2fr) minmax(13rem, 1fr)')
    expect(tradeDesktopGridTemplateColumns(true, false)).toBe('minmax(13rem, 1fr) minmax(0, 3.2fr)')
    expect(tradeDesktopGridTemplateColumns(false, false)).toBe('minmax(0, 1fr)')
    expect(tradeDesktopChartGridColumn(true)).toBe(2)
    expect(tradeDesktopChartGridColumn(false)).toBe(1)
  })
})
