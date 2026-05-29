import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { applyStationKeplrShimSignDefaults } from '../stationExtensionConfig'

describe('applyStationKeplrShimSignDefaults', () => {
  beforeEach(() => {
    vi.stubGlobal('window', {} as Window & typeof globalThis)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('sets preferNoSetFee on station.keplr', () => {
    const keplr: { defaultOptions?: { sign?: { preferNoSetFee?: boolean; preferNoSetMemo?: boolean } } } = {}
    vi.stubGlobal('window', { station: { keplr } } as unknown as Window & typeof globalThis)

    applyStationKeplrShimSignDefaults()

    expect(keplr.defaultOptions).toEqual({
      sign: { preferNoSetFee: true, preferNoSetMemo: true },
    })
  })

  it('no-ops when station shim is missing', () => {
    expect(() => applyStationKeplrShimSignDefaults()).not.toThrow()
  })

  it('no-ops when only window.keplr is present (not Station shim)', () => {
    const keplr = { defaultOptions: { sign: { preferNoSetFee: false } } }
    vi.stubGlobal('window', { keplr } as unknown as Window & typeof globalThis)

    applyStationKeplrShimSignDefaults()
    expect(keplr.defaultOptions.sign.preferNoSetFee).toBe(false)
  })
})
