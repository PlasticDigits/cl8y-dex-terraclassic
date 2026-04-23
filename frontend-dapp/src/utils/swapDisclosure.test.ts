import { describe, it, expect } from 'vitest'
import { getDirectHybridBookSplit, getIndexerHybridExecutionSummary } from './swapDisclosure'

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

describe('getIndexerHybridExecutionSummary', () => {
  it('hides for pool-only and route-only kinds', () => {
    expect(getIndexerHybridExecutionSummary('indexer_pool_lcd').show).toBe(false)
    expect(getIndexerHybridExecutionSummary('indexer_route_only').show).toBe(false)
    expect(getIndexerHybridExecutionSummary(undefined).show).toBe(false)
  })

  it('shows for hybrid LCD kinds', () => {
    const a = getIndexerHybridExecutionSummary('indexer_hybrid_lcd')
    expect(a.show).toBe(true)
    if (a.show) expect(a.degraded).toBe(false)
    const b = getIndexerHybridExecutionSummary('indexer_hybrid_lcd_degraded')
    expect(b.show).toBe(true)
    if (b.show) expect(b.degraded).toBe(true)
  })
})
