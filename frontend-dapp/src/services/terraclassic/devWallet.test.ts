import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./wallet', () => ({
  registerConnectedWallet: vi.fn(),
}))

const HERE = path.dirname(fileURLToPath(import.meta.url))
const REPO_ROOT = path.resolve(HERE, '..', '..', '..', '..')

function localTerraTestMnemonic(): string {
  const text = readFileSync(path.join(REPO_ROOT, 'docker', 'init-chain.sh'), 'utf8')
  const m = text.match(/^TEST_MNEMONIC="([^"]+)"\s*$/m)
  if (!m) {
    throw new Error('TEST_MNEMONIC not found in docker/init-chain.sh')
  }
  return m[1]!
}

describe('devWallet (GitLab #118)', () => {
  beforeEach(() => {
    vi.unstubAllEnvs()
    vi.stubEnv('VITE_DEV_MODE', 'true')
  })
  afterEach(() => {
    vi.unstubAllEnvs()
  })

  it('throws when VITE_DEV_MNEMONIC is empty', async () => {
    vi.stubEnv('VITE_DEV_MNEMONIC', '')
    vi.resetModules()
    const { createDevTerraWallet } = await import('./devWallet')
    expect(() => createDevTerraWallet()).toThrow(/VITE_DEV_MNEMONIC is required/)
  })

  it('constructs a wallet and uses the known localterra address', async () => {
    vi.stubEnv('VITE_DEV_MNEMONIC', localTerraTestMnemonic())
    vi.resetModules()
    const { createDevTerraWallet } = await import('./devWallet')
    const w = createDevTerraWallet()
    expect(w.address).toBe('terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v')
  })
})
