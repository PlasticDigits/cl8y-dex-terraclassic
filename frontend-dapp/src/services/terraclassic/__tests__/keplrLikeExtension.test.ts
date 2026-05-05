import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { WalletName } from '@goblinhunt/cosmes/wallet'

describe('getKeplrLikeExtension (GitLab #127 Station LocalTerra gas)', () => {
  beforeEach(() => {
    vi.stubGlobal('window', {} as Window & typeof globalThis)
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('returns station.keplr for Station', async () => {
    const suggest = vi.fn()
    const stationKeplr = { experimentalSuggestChain: suggest }
    vi.stubGlobal('window', {
      station: { keplr: stationKeplr },
    } as unknown as Window & typeof globalThis)

    const { getKeplrLikeExtension } = await import('../keplrLikeExtension')
    expect(getKeplrLikeExtension(WalletName.STATION)).toBe(stationKeplr)
  })

  it('returns undefined when station.keplr is missing', async () => {
    vi.stubGlobal('window', { station: {} } as unknown as Window & typeof globalThis)
    const { getKeplrLikeExtension } = await import('../keplrLikeExtension')
    expect(getKeplrLikeExtension(WalletName.STATION)).toBeUndefined()
  })
})
