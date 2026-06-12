import { describe, expect, it } from 'vitest'
import { terraBroadcastPendingButtonLabel } from '../terraBroadcastUi'

describe('terraBroadcastPendingButtonLabel (GitLab #305)', () => {
  it('returns idle label when not pending', () => {
    expect(terraBroadcastPendingButtonLabel(null, false, 'Swap', 'Swapping…')).toBe('Swap')
  })

  it('maps phases to retail copy', () => {
    expect(terraBroadcastPendingButtonLabel('signing', true, 'Swap', 'Swapping…')).toBe('Signing…')
    expect(terraBroadcastPendingButtonLabel('broadcasting', true, 'Swap', 'Swapping…')).toBe('Broadcasting…')
    expect(terraBroadcastPendingButtonLabel('confirming', true, 'Swap', 'Swapping…')).toBe('Confirming…')
    expect(terraBroadcastPendingButtonLabel('recovering', true, 'Swap', 'Swapping…')).toBe('Broadcast status unknown…')
  })

  it('falls back when pending without a tracked phase', () => {
    expect(terraBroadcastPendingButtonLabel(null, true, 'Place limit', 'Placing…')).toBe('Placing…')
  })
})
