import { afterEach, describe, expect, it, vi } from 'vitest'
import { shortenTxHashForDisplay } from '../terraExplorer'

const SAMPLE_ADDRESS = 'terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v'
const SAMPLE_TX = 'C8845E73934EC9016F751B65F722DBCFBF167C7C5FC4238E5DF39437451412DB'

async function loadExplorerUtils() {
  return import('../terraExplorer')
}

describe('shortenTxHashForDisplay', () => {
  it('middle-elides long hashes', () => {
    const h = `AAAAAAAA${'0'.repeat(50)}BBBBBB`
    expect(shortenTxHashForDisplay(h)).toBe('AAAAAAAA…BBBBBB')
  })

  it('returns short strings unchanged', () => {
    expect(shortenTxHashForDisplay('abc')).toBe('abc')
  })
})

describe('getExplorerTxUrl', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
  })

  it('uses LocalTerra LCD tx REST for local builds', async () => {
    vi.stubEnv('VITE_NETWORK', 'local')
    vi.resetModules()
    const { getExplorerTxUrl } = await loadExplorerUtils()
    expect(getExplorerTxUrl(SAMPLE_TX)).toBe(`http://localhost:1317/cosmos/tx/v1beta1/txs/${SAMPLE_TX}`)
  })

  it('uses Finder mainnet tx path for mainnet builds', async () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.resetModules()
    const { getExplorerTxUrl } = await loadExplorerUtils()
    expect(getExplorerTxUrl(SAMPLE_TX)).toBe(`https://finder.terraclassic.community/mainnet/tx/${SAMPLE_TX}`)
  })

  it('uses Finder testnet tx path for testnet builds', async () => {
    vi.stubEnv('VITE_NETWORK', 'testnet')
    vi.resetModules()
    const { getExplorerTxUrl } = await loadExplorerUtils()
    expect(getExplorerTxUrl(SAMPLE_TX)).toBe(`https://finder.terra-classic.hexxagon.io/testnet/tx/${SAMPLE_TX}`)
  })

  describe('adversarial input (#430 / SEC-E10)', () => {
    async function loadMainnetExplorer() {
      vi.stubEnv('VITE_NETWORK', 'mainnet')
      vi.resetModules()
      return loadExplorerUtils()
    }

    it('returns null for javascript: prefix instead of a javascript: URL', async () => {
      const { getExplorerTxUrl } = await loadMainnetExplorer()
      expect(getExplorerTxUrl('javascript:alert(1)')).toBeNull()
      expect(getExplorerTxUrl(`javascript:alert(1)//${SAMPLE_TX}`)).toBeNull()
    })

    it('returns null for HTML-special characters in tx hash', async () => {
      const { getExplorerTxUrl } = await loadMainnetExplorer()
      expect(getExplorerTxUrl(`${SAMPLE_TX}<script>`)).toBeNull()
      expect(getExplorerTxUrl(`<script>${SAMPLE_TX}`)).toBeNull()
    })

    it('returns null for empty string', async () => {
      const { getExplorerTxUrl } = await loadMainnetExplorer()
      expect(getExplorerTxUrl('')).toBeNull()
    })

    it('returns null for path-traversal segments', async () => {
      const { getExplorerTxUrl } = await loadMainnetExplorer()
      expect(getExplorerTxUrl('../etc/passwd')).toBeNull()
    })
  })
})

describe('getExplorerAddressUrl', () => {
  afterEach(() => {
    vi.unstubAllEnvs()
  })

  it('uses LocalTerra LCD account REST for local builds', async () => {
    vi.stubEnv('VITE_NETWORK', 'local')
    vi.resetModules()
    const { getExplorerAddressUrl } = await loadExplorerUtils()
    expect(getExplorerAddressUrl(SAMPLE_ADDRESS)).toBe(
      `http://localhost:1317/cosmos/auth/v1beta1/accounts/${SAMPLE_ADDRESS}`
    )
  })

  it('uses Finder mainnet address path for mainnet builds', async () => {
    vi.stubEnv('VITE_NETWORK', 'mainnet')
    vi.resetModules()
    const { getExplorerAddressUrl } = await loadExplorerUtils()
    expect(getExplorerAddressUrl(SAMPLE_ADDRESS)).toBe(
      `https://finder.terraclassic.community/mainnet/address/${SAMPLE_ADDRESS}`
    )
  })

  it('uses Finder testnet address path for testnet builds', async () => {
    vi.stubEnv('VITE_NETWORK', 'testnet')
    vi.resetModules()
    const { getExplorerAddressUrl } = await loadExplorerUtils()
    expect(getExplorerAddressUrl(SAMPLE_ADDRESS)).toBe(
      `https://finder.terra-classic.hexxagon.io/testnet/address/${SAMPLE_ADDRESS}`
    )
  })

  describe('adversarial input (#430 / SEC-E10)', () => {
    async function loadMainnetExplorer() {
      vi.stubEnv('VITE_NETWORK', 'mainnet')
      vi.resetModules()
      return loadExplorerUtils()
    }

    it('returns null for javascript: prefix instead of a javascript: URL', async () => {
      const { getExplorerAddressUrl } = await loadMainnetExplorer()
      expect(getExplorerAddressUrl('javascript:alert(1)')).toBeNull()
    })

    it('returns null for HTML-special characters in address', async () => {
      const { getExplorerAddressUrl } = await loadMainnetExplorer()
      expect(getExplorerAddressUrl(`${SAMPLE_ADDRESS}<script>`)).toBeNull()
      expect(getExplorerAddressUrl(`<script>${SAMPLE_ADDRESS}`)).toBeNull()
    })

    it('returns null for empty string', async () => {
      const { getExplorerAddressUrl } = await loadMainnetExplorer()
      expect(getExplorerAddressUrl('')).toBeNull()
    })
  })
})
