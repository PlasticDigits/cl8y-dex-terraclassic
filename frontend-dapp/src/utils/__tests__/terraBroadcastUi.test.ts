import { describe, it, expect } from 'vitest'
import { terraBroadcastPendingButtonLabel, terraBroadcastPendingStatusMessage } from '../terraBroadcastUi'

describe('terraBroadcastPendingButtonLabel (GitLab #305)', () => {
  it('returns idle label when not pending', () => {
    expect(terraBroadcastPendingButtonLabel(null, false, 'Swap', 'Swapping…')).toBe('Swap')
  })

  it('returns phase-specific labels when pending', () => {
    expect(terraBroadcastPendingButtonLabel('signing', true, 'Swap', 'Swapping…')).toBe('Signing…')
    expect(terraBroadcastPendingButtonLabel('broadcasting', true, 'Swap', 'Swapping…')).toBe('Broadcasting…')
    expect(terraBroadcastPendingButtonLabel('confirming', true, 'Swap', 'Swapping…')).toBe('Confirming…')
    expect(terraBroadcastPendingButtonLabel('recovering', true, 'Swap', 'Swapping…')).toBe('Checking broadcast…')
  })

  it('falls back to pendingFallback when phase is null but pending', () => {
    expect(terraBroadcastPendingButtonLabel(null, true, 'Place limit', 'Placing…')).toBe('Placing…')
  })
})

describe('terraBroadcastPendingStatusMessage (GitLab #359)', () => {
  it('surfaces post-sign unknown copy only during recovering', () => {
    expect(terraBroadcastPendingStatusMessage('recovering')).toContain('Broadcast status unknown')
    expect(terraBroadcastPendingStatusMessage('confirming')).toBeNull()
    expect(terraBroadcastPendingStatusMessage(null)).toBeNull()
  })
})

describe('terraBroadcastPendingStatusMessage signing hints (GitLab #567)', () => {
  it('shows Ledger hint immediately during signing', () => {
    expect(terraBroadcastPendingStatusMessage('signing', { isNanoLedger: true })).toMatch(/LUNA/)
    expect(terraBroadcastPendingStatusMessage('signing', { isNanoLedger: true })).toMatch(/not Cosmos/)
    expect(terraBroadcastPendingStatusMessage('signing', { isNanoLedger: true })).not.toMatch(/330|118/)
  })

  it('does not show Ledger-only text for software Keplr at t=0', () => {
    expect(terraBroadcastPendingStatusMessage('signing', { isNanoLedger: false, signingElapsedMs: 0 })).toBeNull()
  })

  it('shows delayed Keplr hint after the delay window', () => {
    const msg = terraBroadcastPendingStatusMessage('signing', { isNanoLedger: false, signingElapsedMs: 12_000 })
    expect(msg).toMatch(/Approve in Keplr/)
    expect(msg).toMatch(/LUNA/)
  })

  it('keeps recovering copy unchanged', () => {
    expect(terraBroadcastPendingStatusMessage('recovering', { isNanoLedger: true })).toContain(
      'Broadcast status unknown'
    )
  })
})
