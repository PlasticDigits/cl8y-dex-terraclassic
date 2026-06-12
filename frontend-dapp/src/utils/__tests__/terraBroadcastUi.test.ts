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
