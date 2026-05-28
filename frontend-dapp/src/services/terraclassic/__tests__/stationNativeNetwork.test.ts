import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  buildStationLocalNetworkInfo,
  ensureStationLocalNetworkRegistered,
  shouldUseStationNativeLocalNetwork,
} from '../stationNativeNetwork'

describe('stationNativeNetwork (GitLab #207)', () => {
  beforeEach(() => {
    vi.stubGlobal('window', {} as Window & typeof globalThis)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('buildStationLocalNetworkInfo uses localterra and LCD', () => {
    const info = buildStationLocalNetworkInfo('http://localhost:1317')
    expect(info.chainID).toBe('localterra')
    expect(info.lcd).toBe('http://localhost:1317')
    expect(info.coinType).toBe('330')
    expect(info.gasPrices?.uluna).toBeGreaterThan(0)
  })

  it('ensureStationLocalNetworkRegistered skips when addNetwork is missing', async () => {
    vi.stubGlobal('window', { station: { keplr: {} } } as unknown as Window & typeof globalThis)
    await expect(ensureStationLocalNetworkRegistered('http://localhost:1317')).resolves.toBe('skipped')
    expect(shouldUseStationNativeLocalNetwork()).toBe(false)
  })

  it('ensureStationLocalNetworkRegistered calls addNetwork when not present', async () => {
    const hasNetwork = vi.fn().mockResolvedValue(false)
    const addNetwork = vi.fn().mockResolvedValue(true)
    vi.stubGlobal('window', { station: { hasNetwork, addNetwork } } as unknown as Window & typeof globalThis)

    await expect(ensureStationLocalNetworkRegistered('http://localhost:1317')).resolves.toBe('registered')
    expect(addNetwork).toHaveBeenCalledWith(
      expect.objectContaining({ chainID: 'localterra', lcd: 'http://localhost:1317' })
    )
  })

  it('ensureStationLocalNetworkRegistered skips add when hasNetwork is true', async () => {
    const hasNetwork = vi.fn().mockResolvedValue(true)
    const addNetwork = vi.fn()
    vi.stubGlobal('window', { station: { hasNetwork, addNetwork } } as unknown as Window & typeof globalThis)

    await expect(ensureStationLocalNetworkRegistered('http://localhost:1317')).resolves.toBe('already')
    expect(addNetwork).not.toHaveBeenCalled()
  })
})
