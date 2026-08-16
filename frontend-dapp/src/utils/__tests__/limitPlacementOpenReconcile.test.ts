import { describe, expect, it } from 'vitest'
import {
  classifyOpenLimitRow,
  openLimitCancelButtonLabel,
  openLimitCancelEnabled,
  openLimitRowMarker,
  openLimitRowStatusCopy,
  orderIdHasIndexedFill,
  reconcilePlacementRowKind,
} from '@/utils/limitPlacementOpenReconcile'

const base = {
  lcdStatus: undefined as undefined,
  indexerLifecycle: 'active' as const,
  hasIndexedCancellation: false,
  hasIndexedFill: false,
}

describe('classifyOpenLimitRow (GitLab #530)', () => {
  it('I1: LCD Active + indexer active → cancelable ●', () => {
    expect(classifyOpenLimitRow({ ...base, lcdStatus: 'active' })).toBe('cancelable')
    expect(openLimitRowMarker('cancelable')).toBe('●')
  })

  it('I2 / I9: indexed or local cancel → already_cancelled (no fake Cancel)', () => {
    expect(classifyOpenLimitRow({ ...base, hasIndexedCancellation: true })).toBe('already_cancelled')
    expect(classifyOpenLimitRow({ ...base, lcdStatus: 'active', locallyCancelled: true })).toBe('already_cancelled')
    expect(
      openLimitCancelButtonLabel({
        kind: 'already_cancelled',
        isWalletConnected: true,
        isPairPaused: false,
        tradingRestricted: false,
        pending: false,
      })
    ).toBe('Already cancelled')
    expect(
      openLimitCancelEnabled({
        kind: 'already_cancelled',
        isWalletConnected: true,
        isPairPaused: false,
        tradingRestricted: false,
        pending: false,
        hasCancelMutation: true,
      })
    ).toBe(false)
  })

  it('I3 / L21: LCD Unknown + fill evidence → Filled (Unknown ≠ fill alone)', () => {
    expect(classifyOpenLimitRow({ ...base, lcdStatus: 'unknown', hasIndexedFill: true })).toBe('filled')
    expect(classifyOpenLimitRow({ ...base, lcdStatus: 'unknown', hasIndexedFill: false })).toBe('gone')
    expect(openLimitRowStatusCopy('filled')).toBe('Filled')
    expect(openLimitRowStatusCopy('gone')).toBe('No longer on the book')
    expect(openLimitRowMarker('filled')).toBe('○')
  })

  it('I5: LCD ParkedRefund or indexer parked → claim, not Cancel', () => {
    expect(classifyOpenLimitRow({ ...base, lcdStatus: 'parked_refund' })).toBe('claim')
    expect(classifyOpenLimitRow({ ...base, indexerLifecycle: 'parked_expired' })).toBe('claim')
    expect(
      openLimitCancelButtonLabel({
        kind: 'claim',
        isWalletConnected: true,
        isPairPaused: false,
        tradingRestricted: false,
        pending: false,
      })
    ).toBeNull()
  })

  it('L21: LCD query failure (undefined) does not become Unknown — Cancel stays offered', () => {
    expect(classifyOpenLimitRow({ ...base, lcdStatus: undefined })).toBe('cancelable')
  })

  it('AC5: paused / restricted labels are explicit', () => {
    expect(
      openLimitCancelButtonLabel({
        kind: 'cancelable',
        isWalletConnected: true,
        isPairPaused: true,
        tradingRestricted: false,
        pending: false,
      })
    ).toBe('Unavailable (pair paused)')
    expect(
      openLimitCancelButtonLabel({
        kind: 'cancelable',
        isWalletConnected: true,
        isPairPaused: false,
        tradingRestricted: true,
        pending: false,
      })
    ).toBe('Trading restricted')
  })

  it('report-class fixture: indexer active + LCD unknown + fill → Filled', () => {
    const kind = reconcilePlacementRowKind(
      { lifecycle_status: 'active' },
      {
        lcdStatus: 'unknown',
        hasIndexedCancellation: false,
        hasIndexedFill: true,
      }
    )
    expect(kind).toBe('filled')
    expect(orderIdHasIndexedFill([{ order_id: 1 }], 1)).toBe(true)
  })
})
