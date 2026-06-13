import { describe, expect, it } from 'vitest'
import { resolveTerraTxRecoveryDeadlineUnix } from '../terraMsgDeadline'

describe('resolveTerraTxRecoveryDeadlineUnix (GitLab #359)', () => {
  it('reads deadline from top-level swap msg', () => {
    const deadline = resolveTerraTxRecoveryDeadlineUnix(
      [{ contract: 'terra1pair', msg: { swap: { deadline: 1_700_000_100 } } }],
      1_700_000_000
    )
    expect(deadline).toBe(1_700_000_100)
  })

  it('reads deadline from CW20 send inner execute_swap_operations', () => {
    const inner = btoa(JSON.stringify({ execute_swap_operations: { deadline: 1_800_000_000 } }))
    const deadline = resolveTerraTxRecoveryDeadlineUnix(
      [{ contract: 'terra1token', msg: { send: { contract: 'terra1router', amount: '1', msg: inner } } }],
      1_700_000_000
    )
    expect(deadline).toBe(1_800_000_000)
  })

  it('falls back to now + default recovery seconds when no deadline in msgs', () => {
    const now = 1_700_000_000
    const deadline = resolveTerraTxRecoveryDeadlineUnix(
      [{ contract: 'terra1pair', msg: { increase_allowance: { spender: 'x', amount: '1' } } }],
      now
    )
    expect(deadline).toBe(now + 300)
  })
})
