import { describe, expect, it } from 'vitest'
import { evaluateLivePins, isPreF6AssetCodeIdsError, CODE_ID_FROZEN_BANNER } from '../assetCodeIdFreeze'

describe('evaluateLivePins (GitLab #585)', () => {
  it('is tradable when pins match live ids and both are whitelisted', () => {
    expect(
      evaluateLivePins({
        pin0: 10184,
        pin1: 6036,
        live0: 10184,
        live1: 6036,
        whitelisted0: true,
        whitelisted1: true,
      })
    ).toBe('tradable')
  })

  it('is frozen on drift or de-whitelist', () => {
    expect(
      evaluateLivePins({
        pin0: 10184,
        pin1: 6036,
        live0: 9999,
        live1: 6036,
        whitelisted0: true,
        whitelisted1: true,
      })
    ).toBe('frozen')
    expect(
      evaluateLivePins({
        pin0: 10184,
        pin1: 6036,
        live0: 10184,
        live1: 6036,
        whitelisted0: true,
        whitelisted1: false,
      })
    ).toBe('frozen')
  })

  it('classifies pre-1.15.0 GetAssetCodeIds rejects as not freeze evidence', () => {
    expect(isPreF6AssetCodeIdsError('unknown variant `get_asset_code_ids`')).toBe(true)
    expect(isPreF6AssetCodeIdsError('Asset CW20 code_id pins are missing')).toBe(true)
    expect(isPreF6AssetCodeIdsError('timeout')).toBe(false)
  })

  it('banner copy says quotes can still appear', () => {
    expect(CODE_ID_FROZEN_BANNER.toLowerCase()).toContain('quotes can still appear')
  })
})
