import { describe, expect, it } from 'vitest'
import { tryHumanizeTerraTxMessage, stripNestedTransactionFailedPrefixes } from '../humanizeTerraTxError'

describe('stripNestedTransactionFailedPrefixes', () => {
  it('strips a single prefix', () => {
    expect(stripNestedTransactionFailedPrefixes('Transaction failed: out of gas')).toBe('out of gas')
  })

  it('strips repeated prefixes', () => {
    expect(
      stripNestedTransactionFailedPrefixes('Transaction failed: Transaction failed: Transaction failed: out of gas'),
    ).toBe('out of gas')
  })

  it('returns the message unchanged when no prefix is present', () => {
    expect(stripNestedTransactionFailedPrefixes('out of gas')).toBe('out of gas')
  })
})

describe('tryHumanizeTerraTxMessage — existing branches (regression coverage)', () => {
  it('humanizes Max spread assertion errors', () => {
    const raw =
      'failed to execute message; message index: 0: dispatch: submessages: Max spread assertion: actual spread (0.969) exceeds max allowed (0.01): execute wasm contract failed'
    const out = tryHumanizeTerraTxMessage(raw)
    expect(out).not.toBeNull()
    expect(out).toContain('Trade rejected')
    expect(out).toContain('slippage tolerance')
  })

  it('humanizes LimitOrder map key not found errors (binary key)', () => {
    const raw =
      'failed to execute message; message index: 0: type: cl8y_dex_pair::state::LimitOrder; key: [00, 0C, 6C, 69, 6D, 69, 74, 5F, 6F, 72, 64, 65, 72, 73, 00, 00, 00, 00, 00, 00, 00, 03] not found: execute wasm contract failed'
    const out = tryHumanizeTerraTxMessage(raw)
    expect(out).not.toBeNull()
    expect(out).toContain('no longer on the book')
  })
})

describe('tryHumanizeTerraTxMessage — new branches (GitLab #134)', () => {
  describe('contract paused', () => {
    it('matches assert_not_paused rejection', () => {
      const raw =
        'failed to execute message; message index: 0: assert_not_paused: pair contract is paused: execute wasm contract failed'
      const out = tryHumanizeTerraTxMessage(raw)
      expect(out).not.toBeNull()
      expect(out).toContain('paused')
    })

    it('matches generic "contract is paused"', () => {
      const raw = 'Transaction failed: dispatch: submessages: contract is paused'
      const out = tryHumanizeTerraTxMessage(raw)
      expect(out).not.toBeNull()
      expect(out).toContain('paused')
    })
  })

  describe('Unauthorized', () => {
    it('matches admin-gated entrypoint Unauthorized rejection', () => {
      const raw = 'failed to execute message; message index: 0: Unauthorized: execute wasm contract failed'
      const out = tryHumanizeTerraTxMessage(raw)
      expect(out).not.toBeNull()
      expect(out).toContain('permission')
    })

    it('does not false-match on substring "authoriz" inside other words', () => {
      const raw = 'failed to execute message; message index: 0: pre-authorization not found: execute wasm contract failed'
      const out = tryHumanizeTerraTxMessage(raw)
      expect(out).toBeNull()
    })
  })

  describe('Insufficient funds', () => {
    it('matches Insufficient funds for fees', () => {
      const raw = 'failed to execute: Insufficient funds: 0uluna < 100000uluna'
      const out = tryHumanizeTerraTxMessage(raw)
      expect(out).not.toBeNull()
      expect(out).toContain('LUNC')
    })

    it('matches lowercase "insufficient funds"', () => {
      const raw = 'tx failed: insufficient funds in account'
      const out = tryHumanizeTerraTxMessage(raw)
      expect(out).not.toBeNull()
      expect(out).toContain('LUNC')
    })
  })

  describe('out of gas', () => {
    it('matches "out of gas" anywhere in the message', () => {
      const raw = 'Transaction failed: out of gas in location: WriteFlat; gasWanted: 200000, gasUsed: 215000'
      const out = tryHumanizeTerraTxMessage(raw)
      expect(out).not.toBeNull()
      expect(out).toContain('gas')
      expect(out).toContain('Try again')
    })
  })

  describe('deadline exceeded', () => {
    it('matches assert_deadline rejection', () => {
      const raw = 'failed to execute: assert_deadline: block height exceeded order deadline'
      const out = tryHumanizeTerraTxMessage(raw)
      expect(out).not.toBeNull()
      expect(out).toContain('deadline')
    })

    it('matches generic "deadline exceeded"', () => {
      const raw = 'context deadline exceeded'
      const out = tryHumanizeTerraTxMessage(raw)
      expect(out).not.toBeNull()
      expect(out).toContain('deadline')
    })
  })

  describe('InvariantViolation', () => {
    it('matches pending-escrow invariant violation', () => {
      const raw =
        'failed to execute message; message index: 0: InvariantViolation: pending escrow not zero on pool finalize: execute wasm contract failed'
      const out = tryHumanizeTerraTxMessage(raw)
      expect(out).not.toBeNull()
      expect(out).toContain('inconsistency')
    })

    it('matches generic InvariantViolation regardless of suffix', () => {
      const raw = 'InvariantViolation: total liquidity mismatch'
      const out = tryHumanizeTerraTxMessage(raw)
      expect(out).not.toBeNull()
      expect(out).toContain('inconsistency')
    })
  })

  describe('passthrough — unrecognized errors', () => {
    it('returns null for completely unknown chain errors', () => {
      const raw = 'failed to execute message; message index: 0: some weird new error: execute wasm contract failed'
      expect(tryHumanizeTerraTxMessage(raw)).toBeNull()
    })

    it('returns null for empty string', () => {
      expect(tryHumanizeTerraTxMessage('')).toBeNull()
    })

    it('returns null for plain "Transaction failed:" with no inner message', () => {
      expect(tryHumanizeTerraTxMessage('Transaction failed:')).toBeNull()
    })
  })
})
