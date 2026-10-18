import { describe, it, expect } from 'vitest'
import { formatNum } from '@/utils/formatAmount'
import { resolveTradeChartHeadlineUsd } from '../chartHeadlinePrice'

describe('resolveTradeChartHeadlineUsd (#1512)', () => {
  it('formats latest indexed trade USD', () => {
    expect(resolveTradeChartHeadlineUsd('0.982')).toBe(formatNum('0.982', 6))
  })
  it.each([undefined, null, '', '0', '-1', 'Infinity', '1<script>'])(
    'keeps unavailable or invalid current USD %s unavailable',
    (value) => {
      expect(resolveTradeChartHeadlineUsd(value)).toBeNull()
    }
  )
})
