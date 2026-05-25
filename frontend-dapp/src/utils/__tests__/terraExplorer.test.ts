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
})
