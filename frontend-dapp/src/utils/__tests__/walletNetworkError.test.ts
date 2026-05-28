import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import {
  buildWrongNetworkConnectError,
  isWalletExtensionNotInstalledError,
  isWalletWrongNetworkError,
} from '../walletNetworkError'

describe('walletNetworkError (GitLab #207)', () => {
  beforeEach(() => {
    vi.stubEnv('VITE_NETWORK', 'local')
  })

  afterEach(() => {
    vi.unstubAllEnvs()
  })

  it('detects Station wrong-network SDK errors', () => {
    expect(isWalletWrongNetworkError('The requested chain is not available on Station.')).toBe(true)
    expect(isWalletWrongNetworkError('localterra is not available on Station.')).toBe(true)
    expect(isWalletWrongNetworkError('Failed to get public key for localterra')).toBe(true)
    expect(isWalletWrongNetworkError('No wallets connected')).toBe(true)
  })

  it('does not classify chain errors as not-installed', () => {
    expect(
      isWalletExtensionNotInstalledError('Failed to connect Station: The requested chain is not available on Station.')
    ).toBe(false)
  })

  it('detects explicit not-installed messages', () => {
    expect(isWalletExtensionNotInstalledError('Station extension is not installed')).toBe(true)
  })

  it('buildWrongNetworkConnectError names LocalTerra for local builds', () => {
    const msg = buildWrongNetworkConnectError('Station')
    expect(msg).toMatch(/wrong network/i)
    expect(msg).toMatch(/LocalTerra/)
    expect(msg).toMatch(/localterra/)
  })
})
