import { describe, expect, it } from 'vitest'
import type { IndexerPosition } from '@/types'
import {
  formatScaledPosition,
  formatSignedUsd,
  multiplyNumericByTenPow,
  scaleNumericByDecimals,
  sumRealizedPnlUsd,
  traderUsdMarksFromHub,
  TRADER_PNL_EM_DASH,
} from '../traderPositionDisplay'

function pos(partial: Partial<IndexerPosition> & Pick<IndexerPosition, 'pair_address'>): IndexerPosition {
  return {
    asset_0_symbol: 'UST1',
    asset_1_symbol: 'cUSTC',
    asset_0_decimals: 6,
    asset_1_decimals: 6,
    net_position_quote: '0',
    avg_entry_price: '0',
    total_cost_base: '0',
    realized_pnl: '0',
    trade_count: 1,
    ...partial,
  }
}

describe('scaleNumericByDecimals', () => {
  it('turns 6-dec raw 38290000 into 38.29 (GitLab #551)', () => {
    expect(scaleNumericByDecimals('38290000', 6)).toBe('38.29')
  })

  it('handles signed P&L and leading-zero human amounts', () => {
    expect(scaleNumericByDecimals('-38290000', 6)).toBe('-38.29')
    expect(scaleNumericByDecimals('500000', 6)).toBe('0.5')
    expect(scaleNumericByDecimals('-500000', 6)).toBe('-0.5')
  })

  it('humanizes 18-dec 1 token', () => {
    expect(scaleNumericByDecimals('1000000000000000000', 18)).toBe('1')
  })

  it('humanizes 100 USTR (18-dec raw 1e20) from a plain digit string (GitLab #676)', () => {
    expect(scaleNumericByDecimals('100000000000000000000', 18)).toBe('100')
  })

  it('rejects missing / non-integer decimals', () => {
    expect(scaleNumericByDecimals('1000000', -1)).toBeNull()
    expect(scaleNumericByDecimals('abc', 6)).toBeNull()
  })
})

describe('multiplyNumericByTenPow (avg entry)', () => {
  it('same-decimal 6/6 leaves the ratio unchanged', () => {
    expect(multiplyNumericByTenPow('0.00496', 0)).toBe('0.00496')
  })

  it('scales mixed 6/18 raw avg 1.25e-14 to 0.0125', () => {
    expect(multiplyNumericByTenPow('0.0000000000000125', 12)).toBe('0.0125')
  })

  it('scales cLUNC-as-base tens-of-thousands ratio at 6/6 (already human)', () => {
    expect(multiplyNumericByTenPow('25000', 0)).toBe('25000')
  })
})

describe('formatScaledPosition', () => {
  it('labels quote on net position and base on cost / P&L', () => {
    const d = formatScaledPosition(
      pos({
        pair_address: 'terra1pair',
        net_position_quote: '38290000',
        total_cost_base: '190000',
        realized_pnl: '25000000',
        avg_entry_price: '0.00496',
      })
    )
    expect(d.netPosition).toMatch(/38\.29 cUSTC/)
    expect(d.netPosition).not.toMatch(/M/)
    expect(d.costBasis).toMatch(/UST1/)
    expect(d.realizedPnl).toMatch(/\+25 UST1|\+25\.00 UST1/)
    expect(d.avgEntry).toMatch(/UST1 \/ cUSTC/)
    expect(d.avgEntry).not.toMatch(/T$/)
  })

  it('shows 100 human USTR net from raw 1e20 (GitLab #676)', () => {
    const d = formatScaledPosition(
      pos({
        pair_address: 'terra1ustr',
        asset_0_symbol: 'UST1',
        asset_1_symbol: 'USTR',
        asset_0_decimals: 6,
        asset_1_decimals: 18,
        net_position_quote: '100000000000000000000',
        total_cost_base: '1000000',
        avg_entry_price: '0.00000000000001',
        realized_pnl: '0',
      })
    )
    expect(d.netPosition).toMatch(/100(\.0+)? USTR/)
    expect(d.netPosition).not.toMatch(/[0-9]T\b/)
    expect(d.costBasis).toMatch(/1(\.0+)? UST1/)
  })

  it('does not compact mixed-decimal avg entry as T', () => {
    const d = formatScaledPosition(
      pos({
        pair_address: 'terra1ustr',
        asset_1_symbol: 'USTR',
        asset_1_decimals: 18,
        avg_entry_price: '0.0000000000000125',
        realized_pnl: '0',
      })
    )
    expect(d.avgEntry).toMatch(/0\.0125 UST1 \/ USTR/)
    expect(d.avgEntry).not.toMatch(/[0-9]T\b/)
  })

  it('em-dash when decimals are missing (old indexer)', () => {
    const d = formatScaledPosition(
      pos({
        pair_address: 'terra1old',
        asset_0_decimals: undefined,
        asset_1_decimals: undefined,
        net_position_quote: '38290000',
        realized_pnl: '38290000',
      })
    )
    expect(d.netPosition).toBe(TRADER_PNL_EM_DASH)
    expect(d.realizedPnl).toBe(TRADER_PNL_EM_DASH)
    expect(d.avgEntry).toBe(TRADER_PNL_EM_DASH)
  })

  it('converts UST1 P&L to USD via hub ust1, not $1 (GitLab #560)', () => {
    const d = formatScaledPosition(
      pos({
        pair_address: 'terra1pair',
        realized_pnl: '38290000',
      }),
      { ust1Usd: 0.976, ustcUsd: 0.005 }
    )
    expect(d.realizedPnlUsd).toBeCloseTo(38.29 * 0.976, 6)
    expect(d.realizedPnlUsd).not.toBeCloseTo(38.29, 6)
  })

  it('omits UST1 P&L when hub ust1 is missing (not $1)', () => {
    const d = formatScaledPosition(
      pos({
        pair_address: 'terra1pair',
        realized_pnl: '38290000',
      }),
      { ustcUsd: 0.005 }
    )
    expect(d.realizedPnlUsd).toBeNull()
  })

  it('converts USTR P&L via hub ustr, not 2.5× USTC', () => {
    const d = formatScaledPosition(
      pos({
        pair_address: 'terra1ustr',
        asset_0_symbol: 'USTR',
        asset_0_decimals: 18,
        realized_pnl: '1000000000000000000',
      }),
      { ustcUsd: 0.00473, ustrUsd: 0.00879 }
    )
    expect(d.realizedPnlUsd).toBeCloseTo(0.00879, 6)
    expect(d.realizedPnlUsd).not.toBeCloseTo(0.00473 * 2.5, 5)
  })
})

describe('sumRealizedPnlUsd', () => {
  it('sums USD across pairs and omits unknown quotes (not $0)', () => {
    const summary = sumRealizedPnlUsd(
      [
        pos({ pair_address: 'a', realized_pnl: '1000000' }),
        pos({
          pair_address: 'b',
          asset_0_symbol: 'GEMX',
          realized_pnl: '999999999999',
        }),
      ],
      { ust1Usd: 0.976, ustcUsd: 0.005 }
    )
    expect(summary.usd).toBeCloseTo(0.976, 6)
    expect(summary.pricedPairs).toBe(1)
    expect(summary.unpricedPairs).toBe(1)
  })

  it('does not add raw mixed units from two quote tokens', () => {
    const summary = sumRealizedPnlUsd(
      [
        pos({ pair_address: 'ustc', asset_0_symbol: 'cUSTC', realized_pnl: '1000000' }),
        pos({ pair_address: 'ust1', asset_0_symbol: 'UST1', realized_pnl: '1000000' }),
      ],
      { ustcUsd: 0.005, ust1Usd: 0.976 }
    )
    // 1 cUSTC * $0.005 + 1 UST1 * $0.976 = $0.981 — not 2000000 and not $1.005 peg
    expect(summary.usd).toBeCloseTo(0.981, 6)
  })

  it('empty positions are $0; pending and all-unpriced are null', () => {
    expect(sumRealizedPnlUsd([]).usd).toBe(0)
    expect(sumRealizedPnlUsd(undefined).usd).toBeNull()
    expect(
      sumRealizedPnlUsd([pos({ pair_address: 'g', asset_0_symbol: 'GEMX', realized_pnl: '1000000' })]).usd
    ).toBeNull()
  })
})

describe('traderUsdMarksFromHub (GitLab #560)', () => {
  it('reads ust1/ustr/custc from hub-prices and does not invent pegs', () => {
    const marks = traderUsdMarksFromHub(
      {
        prices: [
          { ticker: 'custc', price_usd: '0.00473' },
          { ticker: 'ust1', price_usd: '0.976' },
          { ticker: 'ustr', price_usd: '0.00879' },
        ],
      },
      { ustcUsd: 0.005, luncUsd: 0.0001 }
    )
    expect(marks.ustcUsd).toBeCloseTo(0.00473, 6)
    expect(marks.ust1Usd).toBeCloseTo(0.976, 6)
    expect(marks.ustrUsd).toBeCloseTo(0.00879, 6)
    expect(marks.luncUsd).toBeCloseTo(0.0001, 6)
  })

  it('omits null hub ticks; CEX ustc fills cUSTC only when hub custc is missing', () => {
    const marks = traderUsdMarksFromHub(
      {
        prices: [
          { ticker: 'custc', price_usd: null },
          { ticker: 'ust1', price_usd: null },
          { ticker: 'ustr', price_usd: '0' },
        ],
      },
      { ustcUsd: 0.005, luncUsd: 0.0001 }
    )
    expect(marks.ustcUsd).toBeCloseTo(0.005, 6)
    expect(marks.ust1Usd).toBeNull()
    expect(marks.ustrUsd).toBeNull()
  })
})

describe('formatSignedUsd', () => {
  it('prefixes sign outside the dollar', () => {
    expect(formatSignedUsd(38.29)).toMatch(/^\+\$/)
    expect(formatSignedUsd(-1.5)).toMatch(/^-\$/)
    expect(formatSignedUsd(null)).toBe(TRADER_PNL_EM_DASH)
  })
})
