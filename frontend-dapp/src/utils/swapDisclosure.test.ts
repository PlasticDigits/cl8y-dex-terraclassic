import { describe, it, expect } from 'vitest'
import {
  getDirectHybridBookSplit,
  getDirectHybridSettingsExecutionSummary,
  getIndexerHybridExecutionSummary,
} from './swapDisclosure'

const CW = 'terra1from00000000000000000000000000000001'

describe('getDirectHybridBookSplit', () => {
  it('returns null when not direct or feature off or not CW20', () => {
    expect(
      getDirectHybridBookSplit({
        isDirect: false,
        useHybridBook: true,
        fromToken: CW,
        bookInputHuman: '1',
        rawInputAmount: '1000000',
        hybridMaxMakers: 8,
      })
    ).toBeNull()
    expect(
      getDirectHybridBookSplit({
        isDirect: true,
        useHybridBook: false,
        fromToken: CW,
        bookInputHuman: '1',
        rawInputAmount: '1000000',
        hybridMaxMakers: 8,
      })
    ).toBeNull()
    expect(
      getDirectHybridBookSplit({
        isDirect: true,
        useHybridBook: true,
        fromToken: 'uluna',
        bookInputHuman: '1',
        rawInputAmount: '1000000',
        hybridMaxMakers: 8,
      })
    ).toBeNull()
  })

  it('splits pay into pool and book (6 decimals) and sets willSubmitHybrid', () => {
    const s = getDirectHybridBookSplit({
      isDirect: true,
      useHybridBook: true,
      fromToken: CW,
      bookInputHuman: '0.4',
      rawInputAmount: '1000000',
      hybridMaxMakers: 8,
    })
    expect(s).not.toBeNull()
    expect(s!.bookRaw).toBe('400000')
    expect(s!.poolRaw).toBe('600000')
    expect(s!.willSubmitHybrid).toBe(true)
    expect(s!.bookExceedsPay).toBe(false)
  })

  it('returns null for invalid book leg draft (no BigInt throw)', () => {
    expect(
      getDirectHybridBookSplit({
        isDirect: true,
        useHybridBook: true,
        fromToken: CW,
        bookInputHuman: '4^0',
        rawInputAmount: '1000000',
        hybridMaxMakers: 8,
      })
    ).toBeNull()
  })

  it('marks bookExceedsPay when book > total', () => {
    const s = getDirectHybridBookSplit({
      isDirect: true,
      useHybridBook: true,
      fromToken: CW,
      bookInputHuman: '2',
      rawInputAmount: '1000000',
      hybridMaxMakers: 8,
    })
    expect(s).not.toBeNull()
    expect(s!.bookExceedsPay).toBe(true)
    expect(s!.willSubmitHybrid).toBe(false)
  })
})

describe('getDirectHybridSettingsExecutionSummary (#492)', () => {
  it('hides when split is null', () => {
    expect(getDirectHybridSettingsExecutionSummary(null).show).toBe(false)
  })

  it('hides when hybrid on and manual book leg empty (no add-a-book-leg copy)', () => {
    const s = getDirectHybridBookSplit({
      isDirect: true,
      useHybridBook: true,
      fromToken: CW,
      bookInputHuman: '',
      rawInputAmount: '1000000',
      hybridMaxMakers: 8,
    })
    expect(s).not.toBeNull()
    expect(s!.bookRaw).toBe('0')
    expect(s!.willSubmitHybrid).toBe(false)
    const summary = getDirectHybridSettingsExecutionSummary(s)
    expect(summary.show).toBe(false)
    expect(JSON.stringify(summary)).not.toMatch(/add a book leg/i)
  })

  it('shows hybrid_manual_split when book leg > 0', () => {
    const s = getDirectHybridBookSplit({
      isDirect: true,
      useHybridBook: true,
      fromToken: CW,
      bookInputHuman: '0.4',
      rawInputAmount: '1000000',
      hybridMaxMakers: 8,
    })
    const summary = getDirectHybridSettingsExecutionSummary(s)
    expect(summary.show).toBe(true)
    if (summary.show) {
      expect(summary.variant).toBe('hybrid_manual_split')
      expect(summary.bookHuman).toBe('0.4')
      expect(summary.poolHuman).toBe('0.6')
    }
  })

  it('shows book_exceeds_pay when book > pay', () => {
    const s = getDirectHybridBookSplit({
      isDirect: true,
      useHybridBook: true,
      fromToken: CW,
      bookInputHuman: '2',
      rawInputAmount: '1000000',
      hybridMaxMakers: 8,
    })
    const summary = getDirectHybridSettingsExecutionSummary(s)
    expect(summary.show).toBe(true)
    if (summary.show) {
      expect(summary.variant).toBe('book_exceeds_pay')
      expect(summary.line).toMatch(/larger than your pay/i)
      expect(summary.line).not.toMatch(/add a book leg/i)
    }
  })

  it('shows max_makers_blocked when book > 0 but makers < 1', () => {
    const s = getDirectHybridBookSplit({
      isDirect: true,
      useHybridBook: true,
      fromToken: CW,
      bookInputHuman: '0.4',
      rawInputAmount: '1000000',
      hybridMaxMakers: 0,
    })
    expect(s!.willSubmitHybrid).toBe(false)
    const summary = getDirectHybridSettingsExecutionSummary(s)
    expect(summary.show).toBe(true)
    if (summary.show) {
      expect(summary.variant).toBe('max_makers_blocked')
      expect(summary.line).toMatch(/max distinct makers/i)
      expect(summary.line).not.toMatch(/add a book leg/i)
    }
  })
})

describe('getIndexerHybridExecutionSummary', () => {
  it('hides for pool-only and route-only kinds', () => {
    expect(getIndexerHybridExecutionSummary('indexer_pool_lcd').show).toBe(false)
    expect(getIndexerHybridExecutionSummary('indexer_route_only').show).toBe(false)
    expect(getIndexerHybridExecutionSummary(undefined).show).toBe(false)
  })

  it('shows for hybrid LCD kinds with retail copy (#414)', () => {
    const db = getIndexerHybridExecutionSummary('indexer_hybrid_db')
    const lcd = getIndexerHybridExecutionSummary('indexer_hybrid_lcd')
    expect(db.show).toBe(true)
    expect(lcd.show).toBe(true)
    if (db.show) {
      expect(db.degraded).toBe(false)
      expect(db.title).toBe('Limit book + pool')
      expect(db.line).not.toMatch(/Postgres|simulate_swap|LCD/i)
    }
    if (lcd.show) expect(lcd.degraded).toBe(false)
    const b = getIndexerHybridExecutionSummary('indexer_hybrid_lcd_degraded')
    expect(b.show).toBe(true)
    if (b.show) expect(b.degraded).toBe(true)
  })
})
