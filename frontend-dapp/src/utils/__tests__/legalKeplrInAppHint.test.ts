import { describe, expect, it } from 'vitest'
import { shouldShowLegalKeplrInAppHint } from '../legalKeplrInAppHint'

describe('shouldShowLegalKeplrInAppHint (GitLab #554)', () => {
  it('shows only when unsigned and Keplr is not injected', () => {
    expect(shouldShowLegalKeplrInAppHint({ hasKeplrExtension: false, signedLatest: false })).toBe(true)
    expect(shouldShowLegalKeplrInAppHint({ hasKeplrExtension: true, signedLatest: false })).toBe(false)
    expect(shouldShowLegalKeplrInAppHint({ hasKeplrExtension: false, signedLatest: true })).toBe(false)
    expect(shouldShowLegalKeplrInAppHint({ hasKeplrExtension: false, signedLatest: null })).toBe(false)
  })
})
