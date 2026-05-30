import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  applyStationKeplrShimSignDefaults,
  prepareStationExtensionForTerraClassicSign,
} from '../stationExtensionConfig'

vi.mock('../stationNativeNetwork', () => ({
  ensureStationLocalNetworkRegistered: vi.fn().mockResolvedValue('updated'),
  shouldUseStationNativeLocalNetwork: vi.fn(() => true),
}))

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

describe('prepareStationExtensionForTerraClassicSign (GitLab #127)', () => {
  beforeEach(() => {
    vi.stubGlobal('window', {
      station: { keplr: {}, addNetwork: vi.fn().mockResolvedValue(true) },
    } as unknown as Window & typeof globalThis)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('sets shim defaults and wallet gas price before sign', async () => {
    const setGasPrice = vi.fn()
    await prepareStationExtensionForTerraClassicSign({ setGasPrice } as never)
    expect(setGasPrice).toHaveBeenCalledWith(expect.objectContaining({ denom: 'uluna' }))
  })
})
