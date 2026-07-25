import { describe, expect, it } from 'vitest'
import {
  clearWalletCachedSequence,
  extractExpectedAccountSequence,
  isAccountSequenceMismatchError,
  planAccountSequenceRetry,
  setWalletCachedSequence,
} from '../terraAccountSequence'

describe('terraAccountSequence (GitLab #499)', () => {
  describe('isAccountSequenceMismatchError', () => {
    it('matches standard Cosmos code-32 copy', () => {
      expect(
        isAccountSequenceMismatchError(
          new Error('account sequence mismatch, expected 133, got 121: incorrect account sequence')
        )
      ).toBe(true)
    })

    it('rejects unrelated errors', () => {
      expect(isAccountSequenceMismatchError(new Error('out of gas'))).toBe(false)
      expect(isAccountSequenceMismatchError('not an error')).toBe(false)
    })
  })

  describe('extractExpectedAccountSequence', () => {
    it('parses expected sequence from CheckTx message', () => {
      expect(
        extractExpectedAccountSequence(
          new Error('account sequence mismatch, expected 133, got 121: incorrect account sequence')
        )
      ).toBe(133n)
    })

    it('returns null when the message is not parseable', () => {
      expect(extractExpectedAccountSequence(new Error('incorrect account sequence'))).toBeNull()
    })
  })

  describe('planAccountSequenceRetry', () => {
    it('applies expected sequence and prefers cache on retry', () => {
      const wallet = { sequence: 121n }
      const plan = planAccountSequenceRetry(
        wallet as never,
        new Error('account sequence mismatch, expected 133, got 121: incorrect account sequence')
      )
      expect(plan).toEqual({ useCachedSequence: true })
      expect(wallet.sequence).toBe(133n)
    })

    it('clears cache when mismatch is unparseable so retry refreshes from chain', () => {
      const wallet = { sequence: 5n }
      const plan = planAccountSequenceRetry(wallet as never, new Error('incorrect account sequence'))
      expect(plan).toEqual({ useCachedSequence: false })
      expect(wallet.sequence).toBeUndefined()
    })

    it('returns null for non-sequence errors', () => {
      const wallet = { sequence: 1n }
      expect(planAccountSequenceRetry(wallet as never, new Error('out of gas'))).toBeNull()
      expect(wallet.sequence).toBe(1n)
    })
  })

  describe('set/clearWalletCachedSequence', () => {
    it('mutates wallet sequence cache', () => {
      const wallet = { sequence: 1n }
      setWalletCachedSequence(wallet as never, 9n)
      expect(wallet.sequence).toBe(9n)
      clearWalletCachedSequence(wallet as never)
      expect(wallet.sequence).toBeUndefined()
    })
  })
})
