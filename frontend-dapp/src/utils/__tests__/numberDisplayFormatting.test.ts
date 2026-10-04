import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import {
  formatHumanAmountGrouped,
  formatNum,
  formatPairPrice,
  formatPairStatsVolume,
  formatTokenAmount,
  formatTokenAmountAbbrev,
  formatTokenAmountGrouped,
  fromRawAmount,
  toRawAmount,
} from '../formatAmount'
import { writeNumberDisplayMode } from '../numberDisplayPreference'
import {
  formatProtocolCount,
  formatProtocolFdusdOut,
  formatProtocolOracleUsd,
  formatProtocolPct,
  formatProtocolUsd,
} from '../formatProtocolStats'
import { formatChartsPairTokenVolume, formatTwapHumanPrice } from '../chartsPairStats'
import { formatIndexedVolumeUsd, formatPairListVolumeUsd } from '../chartsOverviewStats'
import { formatTapeAmount, formatTapePrice } from '../tradeTapeDisplay'
import { limitOrderNativeGasInsufficientMessage } from '../limitOrderNativeGasBalanceGate'
import { provideLiquidityCw20NativeGasInsufficientMessage } from '../provideLiquidityNativeGasBalanceGate'
import { swapNativeGasInsufficientMessage } from '../swapNativeGasBalanceGate'

beforeEach(() => localStorage.clear())
afterEach(() => localStorage.clear())

describe('Compact and Full grouped display (#1259)', () => {
  it.each([
    [13230, 4, '13.23K', '13,230'],
    [1234567, 4, '1.235M', '1,234,567'],
    [5500000000, 3, '5.50B', '5,500,000,000'],
    [2000000000000, 3, '2.00T', '2,000,000,000,000'],
    ['1234567.89000', 4, '1.235M', '1,234,567.89'],
    [-1500, 3, '-1.50K', '-1,500'],
    [0, 4, '0', '0'],
  ] as const)('formats %s after humanization', (value, digits, compact, full) => {
    expect(formatNum(value, digits, 'compact')).toBe(compact)
    expect(formatNum(value, digits, 'full')).toBe(full)
  })

  it('groups raw balances exactly, including 18-dec dust and fractional values above 2^53', () => {
    expect(formatTokenAmount('9007199254740993123456', 6, 4, 'full')).toBe('9,007,199,254,740,993.123456')
    expect(formatTokenAmountAbbrev('9007199254740993123456', 6, 4, 'full')).toBe('9,007,199,254,740,993.123456')
    expect(formatTokenAmountGrouped('1', 18)).toBe('0.000000000000000001')
    expect(formatTokenAmountGrouped('1234567', 0)).toBe('1,234,567')
    expect(formatTokenAmountGrouped('-1500000', 6)).toBe('-1.5')
    expect(formatHumanAmountGrouped('001234567.8900')).toBe('1,234,567.89')
  })

  it('uses the shared preference across balances, volume, tape, tiers, Protocol and Pool', () => {
    writeNumberDisplayMode('full')
    const raw = '1234567890000000000000000'
    expect(formatTokenAmount(raw, 18)).toBe('1,234,567.89')
    expect(formatTokenAmountAbbrev(raw, 18)).toBe('1,234,567.89')
    expect(formatChartsPairTokenVolume(raw, 18)).toBe('1,234,567.89')
    expect(formatTapeAmount(raw, 18, 'USTR')).toBe('1,234,567.89 USTR')
    expect(formatProtocolUsd('1234567.89')).toBe('$1,234,567.89')
    expect(formatProtocolCount(1234567)).toBe('1,234,567')
    expect(formatProtocolPct(1234.5)).toBe('+1,234.5%')
    expect(formatIndexedVolumeUsd('1234567.89', 1)).toBe('$1,234,567.89')
    expect(formatPairListVolumeUsd('1234567.89')).toBe('$1,234,567.89')
    writeNumberDisplayMode('compact')
    expect(formatTokenAmount(raw, 18)).toBe('1.235M')
    expect(formatPairListVolumeUsd('1234567.89')).toBe('$1.235M')
  })

  it.each(['compact', 'full'] as const)('keeps prices, inputs, signing digits and guards independent in %s', (mode) => {
    writeNumberDisplayMode(mode)
    expect(formatPairPrice('1234567', 6)).toBe('1,234,567')
    expect(formatTapePrice('1234567', false)).toBe('1,234,567')
    expect(formatProtocolOracleUsd('1234567.890123')).toBe('$1,234,568')
    expect(formatProtocolFdusdOut('1234567.890123')).toBe('1,234,568')
    expect(formatProtocolOracleUsd('0.123456789')).toBe('$0.123457')
    expect(formatTwapHumanPrice('79718100000000', 6, 18)).toBe('79.7181')
    expect(toRawAmount('1000.5', 18)).toBe('1000500000000000000000')
    expect(fromRawAmount('1000500000000000000000', 18)).toBe('1000.5')
    expect(formatTokenAmountGrouped('1000500000000000000000', 18)).toBe('1,000.5')
    for (const message of [
      limitOrderNativeGasInsufficientMessage,
      provideLiquidityCw20NativeGasInsufficientMessage,
      swapNativeGasInsufficientMessage,
    ]) {
      expect(message(1234567890n)).toContain('1,234.56789 LUNC')
    }
    for (const decimals of [undefined, null, -1, 19, 6.5, NaN]) {
      expect(formatPairStatsVolume('1000000', decimals)).toBe('—')
      expect(formatTokenAmountGrouped('1000000', decimals)).toBe('—')
    }
    for (const raw of ['<img>', '1.5', '0x100', '1e18', '9'.repeat(1000)]) {
      expect(formatPairStatsVolume(raw, 18)).toBe('—')
      expect(formatTokenAmountGrouped(raw, 18)).toBe('—')
    }
    expect(formatProtocolUsd(Infinity)).toBe('—')
  })

  it('keeps the 24-character volume display cap in Full', () => {
    writeNumberDisplayMode('full')
    expect(formatPairStatsVolume('9'.repeat(78), 18)).toBe('—')
    expect(formatChartsPairTokenVolume('9'.repeat(78), 18)).toBe('—')
    expect(formatPairStatsVolume('123456789012345678901', 0)).toBe('—')
    expect(formatPairStatsVolume('1234567890000', 6)).toBe('1,234,567.89')
  })
})
