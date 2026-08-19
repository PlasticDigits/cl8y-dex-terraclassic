import { describe, expect, it } from 'vitest'
import {
  isTerraTxSignStallMessage,
  isTerraTxTimeoutMessage,
  TERRA_TX_BROADCAST_TIMEOUT_MESSAGE,
  TERRA_TX_SIGN_STALL_KEPLR_MESSAGE,
  TERRA_TX_SIGN_STALL_LEDGER_MESSAGE,
  TERRA_TX_SIGNING_KEPLR_DELAYED_HINT,
  TERRA_TX_SIGNING_LEDGER_HINT,
  terraTxSignStallMessage,
} from '../terraTxTimeout'

describe('Keplr/Ledger sign-stall copy (GitLab #567)', () => {
  it('is distinct from the #173 broadcast timeout message', () => {
    expect(TERRA_TX_SIGN_STALL_LEDGER_MESSAGE).not.toBe(TERRA_TX_BROADCAST_TIMEOUT_MESSAGE)
    expect(TERRA_TX_SIGN_STALL_KEPLR_MESSAGE).not.toBe(TERRA_TX_BROADCAST_TIMEOUT_MESSAGE)
    expect(TERRA_TX_SIGN_STALL_LEDGER_MESSAGE).not.toMatch(/check your connection/i)
    expect(TERRA_TX_SIGN_STALL_KEPLR_MESSAGE).not.toMatch(/check your connection/i)
  })

  it('passes through handleBroadcastError as a timeout-class message', () => {
    expect(isTerraTxTimeoutMessage(TERRA_TX_SIGN_STALL_LEDGER_MESSAGE)).toBe(true)
    expect(isTerraTxTimeoutMessage(TERRA_TX_SIGN_STALL_KEPLR_MESSAGE)).toBe(true)
    expect(isTerraTxSignStallMessage(TERRA_TX_BROADCAST_TIMEOUT_MESSAGE)).toBe(false)
  })

  it('picks Ledger vs software copy without coin-type jargon', () => {
    expect(terraTxSignStallMessage(true)).toBe(TERRA_TX_SIGN_STALL_LEDGER_MESSAGE)
    expect(terraTxSignStallMessage(false)).toBe(TERRA_TX_SIGN_STALL_KEPLR_MESSAGE)
    expect(TERRA_TX_SIGNING_LEDGER_HINT).not.toMatch(/330|118/)
    expect(TERRA_TX_SIGNING_KEPLR_DELAYED_HINT).not.toMatch(/330|118/)
  })
})
